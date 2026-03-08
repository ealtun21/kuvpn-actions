use crate::dsid::{run_login_and_get_dsid, LoginConfig};
use crate::openconnect::{
    get_openconnect_pid, is_openconnect_running, is_vpn_interface_up, kill_process,
    OpenConnectRunner, VpnProcess,
};
use crate::utils::{CancellationToken, CredentialsProvider};
use std::io::{BufRead, BufReader, Read};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Error,
}

/// Parsed log message from the "Level|message" format used by the session log channel.
#[derive(Debug, Clone)]
pub struct ParsedLog {
    pub level: log::Level,
    pub message: String,
}

impl ParsedLog {
    /// Parses a "Level|message" string. Returns None if format is invalid.
    pub fn parse(raw: &str) -> Option<Self> {
        let (level_str, message) = raw.split_once('|')?;
        let level = match level_str {
            "Error" => log::Level::Error,
            "Warn" => log::Level::Warn,
            "Info" => log::Level::Info,
            "Debug" => log::Level::Debug,
            "Trace" => log::Level::Trace,
            _ => return None,
        };
        Some(Self {
            level,
            message: message.to_string(),
        })
    }

    /// Returns a short prefix for display (e.g. "ERR", "INF").
    pub fn prefix(&self) -> &'static str {
        match self.level {
            log::Level::Error => "ERR",
            log::Level::Warn => "WRN",
            log::Level::Info => "INF",
            log::Level::Debug => "DBG",
            log::Level::Trace => "TRC",
        }
    }
}

/// Controls how traffic is routed through the VPN tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelMode {
    /// All traffic (0.0.0.0/0) is routed through the tunnel.
    Full,
    /// User supplies their own vpnc-script. `None` passes no `--script` flag
    /// (openconnect falls back to its built-in defaults).
    Manual(Option<String>),
}

#[derive(Clone)]
pub struct SessionConfig {
    pub url: String,
    pub domain: String,
    pub user_agent: String,
    pub headless: bool,
    pub no_auto_login: bool,
    pub email: Option<String>,
    pub openconnect_path: String,
    pub escalation_tool: Option<String>,
    pub interface_name: String,
    /// Controls routing and DNS setup for the VPN tunnel.
    /// On Unix: Split/Full generate a vpnc-script at runtime; Manual passes a user path.
    /// On Windows: only Full/Split distinction is used (no script support).
    pub tunnel_mode: TunnelMode,
}

/// Prompts for the sudo/pkexec password if the chosen escalation tool requires
/// one and no `SUDO_ASKPASS` helper is available.
///
/// Returns `Ok(Some(pw))` on success, `Ok(None)` if no password is needed,
/// or `Err(())` if the user cancelled.
#[cfg(unix)]
fn resolve_sudo_password(
    escalation_tool: &Option<String>,
    provider: &dyn CredentialsProvider,
    cancel_token: &CancellationToken,
    log: &dyn Fn(String),
) -> Result<Option<String>, ()> {
    use crate::openconnect::{
        find_askpass, resolve_escalation_tool, tool_requires_password, verify_escalation_password,
    };

    let tool = resolve_escalation_tool(escalation_tool);
    if let Some(ref tool_name) = tool {
        if tool_requires_password(tool_name) && find_askpass().is_none() {
            let mut wrong_password = false;
            let pw = loop {
                let prompt = if wrong_password {
                    format!(
                        "Wrong password — please try again. \
                         Enter your {} password to start the VPN tunnel",
                        tool_name
                    )
                } else {
                    format!("Enter your {} password to start the VPN tunnel", tool_name)
                };
                log(format!(
                    "Info|{} requires a password. Prompting...",
                    tool_name
                ));
                let entered = match provider.request_password(&prompt) {
                    Some(v) => v,
                    None => return Err(()), // prompt dismissed or cancelled
                };
                if entered.is_empty() && cancel_token.is_cancelled() {
                    return Err(());
                }
                match verify_escalation_password(tool_name, &entered) {
                    Some(true) => break entered,          // verified correct
                    Some(false) => wrong_password = true, // re-prompt
                    None => break entered,                // unverifiable — let sudo decide
                }
            };
            return Ok(Some(pw));
        }
    }
    Ok(None)
}

/// Spawns a thread that reads lines from `stream` and sends them to the log channel
/// prefixed with `level` (e.g. "Info" or "Warn").
fn spawn_stream_reader<R: Read + Send + 'static>(
    stream: R,
    level: &'static str,
    logs_tx: Arc<Mutex<Option<crossbeam_channel::Sender<String>>>>,
) {
    thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            if let Ok(guard) = logs_tx.lock() {
                if let Some(tx) = guard.as_ref() {
                    let _ = tx.send(format!("{}|{}", level, line));
                }
            }
        }
    });
}

struct SessionThread {
    config: SessionConfig,
    status: Arc<Mutex<ConnectionStatus>>,
    cancel_token: CancellationToken,
    last_error: Arc<Mutex<Option<String>>>,
    error_category: Arc<Mutex<Option<crate::error::ErrorCategory>>>,
    logs_tx: Arc<Mutex<Option<crossbeam_channel::Sender<String>>>>,
    browser_pid: Arc<Mutex<Option<u32>>>,
    /// Tracks when the VPN connected so we can record session duration.
    connected_at: Option<Instant>,
    /// Cached sudo/pkexec password resolved during `start_openconnect`.
    #[cfg(unix)]
    sudo_password: Option<String>,
    /// Holds the generated vpnc-script temp file alive for the duration of the session.
    /// Dropped (and the file deleted) when the session ends or reconnects.
    #[cfg(unix)]
    active_script: Option<crate::openconnect::unix::TempScript>,
}

impl SessionThread {
    fn from_session(s: &VpnSession) -> Self {
        Self {
            config: s.config.clone(),
            status: Arc::clone(&s.status),
            cancel_token: s.cancel_token.clone(),
            last_error: Arc::clone(&s.last_error),
            error_category: Arc::clone(&s.error_category),
            logs_tx: Arc::clone(&s.logs_tx),
            browser_pid: Arc::clone(&s.browser_pid),
            connected_at: None,
            #[cfg(unix)]
            sudo_password: None,
            #[cfg(unix)]
            active_script: None,
        }
    }

    fn send_log(&self, msg: impl Into<String>) {
        if let Some(tx) = self.logs_tx.lock().expect("session mutex poisoned").as_ref() {
            if tx.send(msg.into()).is_err() {
                log::debug!("Log channel closed; message dropped");
            }
        }
    }

    fn set_status(&self, s: ConnectionStatus) {
        *self.status.lock().expect("session mutex poisoned") = s;
    }

    fn set_conn_error(&self, msg: &str) {
        *self.status.lock().expect("session mutex poisoned") = ConnectionStatus::Error;
        *self.last_error.lock().expect("session mutex poisoned") = Some(msg.to_string());
        *self.error_category.lock().expect("session mutex poisoned") = Some(crate::error::ErrorCategory::Connection);
        self.send_log(format!("Error|{}", msg));
    }

    fn clear_browser_pid(&self) {
        *self.browser_pid.lock().expect("session mutex poisoned") = None;
    }

    fn handle_login_error(&self, e: anyhow::Error) {
        let mut status = self.status.lock().expect("session mutex poisoned");
        if *status == ConnectionStatus::Disconnecting {
            *status = ConnectionStatus::Disconnected;
        } else {
            let category = e
                .downcast_ref::<crate::error::AuthError>()
                .map(|ae| ae.category())
                .unwrap_or(crate::error::ErrorCategory::Authentication);
            *status = ConnectionStatus::Error;
            *self.last_error.lock().expect("session mutex poisoned") = Some(e.to_string());
            *self.error_category.lock().expect("session mutex poisoned") = Some(category);
            self.send_log(format!("Error|{}", e));
        }
    }

    fn run(mut self, provider: Arc<dyn CredentialsProvider>) {
        const MAX_RETRIES: u32 = 3;
        let mut attempt = 0u32;
        // Duration (secs) of the session that just ended unexpectedly; attached
        // to the next Reconnected history event so history shows how long each
        // segment lasted before dropping.
        let mut prev_duration: Option<u64> = None;

        loop {
            let is_reconnect = attempt > 0;
            let dropped_after = if self.is_vpn_connected() {
                self.send_log("Info|VPN interface already active, monitoring...");
                self.set_status(ConnectionStatus::Connected);
                self.run_watchdog(None, is_reconnect, prev_duration)
            } else {
                match self.launch_vpn(&provider) {
                    Ok(Some(proc)) => self.run_watchdog(Some(proc), is_reconnect, prev_duration),
                    Ok(None) => None, // user cancelled
                    Err(_) => None,   // auth/launch error
                }
            };

            // Retry on unexpected disconnect unless user cancelled or retries exhausted.
            if !self.cancel_token.is_cancelled() && dropped_after.is_some() && attempt < MAX_RETRIES
            {
                attempt += 1;
                prev_duration = dropped_after;
                self.set_status(ConnectionStatus::Connecting);
                *self.last_error.lock().expect("session mutex poisoned") = None;
                *self.error_category.lock().expect("session mutex poisoned") = None;
                self.send_log(format!(
                    "Info|Reconnecting... (attempt {}/{})",
                    attempt, MAX_RETRIES
                ));
                // Brief delay before re-attempting so the gateway isn't hammered
                // and the user has a visible window to cancel.
                for _ in 0..30 {
                    if self.cancel_token.is_cancelled() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                continue;
            }
            if dropped_after.is_some() && attempt >= MAX_RETRIES {
                self.send_log(format!(
                    "Warn|All {} reconnect attempts exhausted. Giving up.",
                    MAX_RETRIES
                ));
            }
            break;
        }
        self.cleanup(attempt);
    }

    fn is_vpn_connected(&self) -> bool {
        is_vpn_interface_up(&self.config.interface_name) || is_openconnect_running()
    }

    fn launch_vpn(
        &mut self,
        provider: &Arc<dyn CredentialsProvider>,
    ) -> Result<Option<VpnProcess>, ()> {
        self.send_log("Info|Accessing campus gateway...");
        let dsid = self.acquire_dsid(provider)?;

        if self.cancel_token.is_cancelled() {
            self.set_status(ConnectionStatus::Disconnected);
            return Ok(None);
        }

        self.send_log("Info|Initializing tunnel...");
        thread::sleep(Duration::from_millis(100));

        let mut proc = self.start_openconnect(dsid, provider)?;
        self.spawn_log_readers(&mut proc);
        Ok(Some(proc))
    }

    fn acquire_dsid(&self, provider: &Arc<dyn CredentialsProvider>) -> Result<String, ()> {
        let login_config = LoginConfig {
            headless: self.config.headless,
            url: self.config.url.clone(),
            domain: self.config.domain.clone(),
            user_agent: self.config.user_agent.clone(),
            no_auto_login: self.config.no_auto_login,
            email: self.config.email.clone(),
        };

        let result = run_login_and_get_dsid(
            &login_config,
            provider.as_ref(),
            Some(self.cancel_token.clone()),
            Some(Arc::clone(&self.browser_pid)),
        );

        self.clear_browser_pid();

        // If browser automation saved a diagnostic bundle, forward its path over
        // the structured log channel so the CLI/GUI can surface it to the user.
        let diag_path = crate::diagnostics::PENDING_DIAG_PATH.with(|cell| cell.borrow_mut().take());
        if let Some(path) = diag_path {
            self.send_log(format!("Diagnostic|{}", path.display()));
        }

        result.map_err(|e| self.handle_login_error(e))
    }

    /// Resolves the vpnc-script path for this session.
    /// On Unix: generates a temp script for Full mode or returns the Manual path.
    /// On non-Unix: always returns `None` (no script support).
    #[cfg(unix)]
    fn resolve_vpnc_script(&mut self) -> Result<Option<String>, ()> {
        use crate::openconnect::unix::generate_vpnc_script;
        let verbose = log::max_level() >= log::LevelFilter::Debug;
        match &self.config.tunnel_mode {
            TunnelMode::Manual(path) => {
                if verbose {
                    self.send_log(format!(
                        "Debug|vpnc-script (manual): {}",
                        path.as_deref().unwrap_or("<none — openconnect built-in>")
                    ));
                }
                Ok(path.clone())
            }
            TunnelMode::Full => {
                let script =
                    generate_vpnc_script().map_err(|e| self.set_conn_error(&e.to_string()))?;
                let path = script.path_str().map(str::to_string);
                if verbose {
                    if let Some(ref p) = path {
                        self.send_log(format!("Debug|Generated vpnc-script: {}", p));
                        if let Ok(content) = std::fs::read_to_string(p) {
                            self.send_log(format!("Debug|Script content:\n{}", content));
                        }
                    }
                }
                self.active_script = Some(script);
                Ok(path)
            }
        }
    }

    #[cfg(not(unix))]
    fn resolve_vpnc_script(&mut self) -> Result<Option<String>, ()> {
        Ok(None)
    }

    /// Prompts for and verifies a sudo/pkexec password when needed.
    /// On Unix: delegates to `resolve_sudo_password`.
    /// On non-Unix: always returns `None`.
    #[cfg(unix)]
    fn resolve_sudo_pw(
        &mut self,
        provider: &Arc<dyn CredentialsProvider>,
    ) -> Result<Option<String>, ()> {
        let log_fn = |m| self.send_log(m);
        let pw = resolve_sudo_password(
            &self.config.escalation_tool,
            provider.as_ref(),
            &self.cancel_token,
            &log_fn,
        )
        .map_err(|_| self.set_status(ConnectionStatus::Disconnected))?;
        self.sudo_password = pw.clone();
        Ok(pw)
    }

    #[cfg(not(unix))]
    fn resolve_sudo_pw(
        &mut self,
        _provider: &Arc<dyn CredentialsProvider>,
    ) -> Result<Option<String>, ()> {
        Ok(None)
    }

    fn start_openconnect(
        &mut self,
        dsid: String,
        provider: &Arc<dyn CredentialsProvider>,
    ) -> Result<VpnProcess, ()> {
        let verbose = log::max_level() >= log::LevelFilter::Debug;

        let custom_script = self.resolve_vpnc_script()?;

        let runner = OpenConnectRunner::locate(
            &self.config.openconnect_path,
            self.config.interface_name.to_string(),
            self.config.escalation_tool.clone(),
            custom_script,
        )
        .ok_or_else(|| {
            self.set_conn_error(&format!(
                "Could not locate openconnect at '{}'. Please install openconnect or set the correct path.",
                self.config.openconnect_path
            ));
        })?;

        if verbose {
            self.send_log(format!("Debug|Tunnel mode: {:?}", self.config.tunnel_mode));
            self.send_log(format!(
                "Debug|openconnect: {} | script: {} | verbose: true",
                runner.path.display(),
                runner.custom_script.as_deref().unwrap_or("<none>"),
            ));
        }

        let sudo_password = self.resolve_sudo_pw(provider)?;

        runner
            .execute(
                dsid,
                self.config.url.clone(),
                Stdio::piped(),
                Stdio::piped(),
                sudo_password,
                matches!(self.config.tunnel_mode, TunnelMode::Full),
                verbose,
            )
            .map_err(|e| self.set_conn_error(&e.to_string()))
    }

    fn spawn_log_readers(&self, proc: &mut VpnProcess) {
        // In debug/trace mode openconnect runs with --verbose; label its stdout
        // as Debug so the flood of detail doesn't pollute the Info console view.
        let oc_stdout_level = if log::max_level() >= log::LevelFilter::Debug {
            "Debug"
        } else {
            "Info"
        };
        if let VpnProcess::Unix(ref mut child) = proc {
            if let Some(stdout) = child.stdout.take() {
                spawn_stream_reader(stdout, oc_stdout_level, Arc::clone(&self.logs_tx));
            }
            if let Some(stderr) = child.stderr.take() {
                spawn_stream_reader(stderr, "Warn", Arc::clone(&self.logs_tx));
            }
        }
    }

    /// Watches the VPN tunnel until it stops.
    ///
    /// Returns `Some(duration_secs)` when the tunnel was successfully established
    /// and then died unexpectedly (eligible for reconnect; duration = how long it
    /// was connected this cycle).  Returns `None` for user-initiated cancellations
    /// and launch-phase errors.
    fn run_watchdog(
        &mut self,
        mut process: Option<VpnProcess>,
        is_reconnect: bool,
        prev_duration: Option<u64>,
    ) -> Option<u64> {
        let start_time = Instant::now();
        let mut connected_detected = process.is_none();
        let timeout = Duration::from_secs(30);

        loop {
            if self.cancel_token.is_cancelled() {
                if let Some(ref mut p) = process {
                    self.send_log("Info|Disconnecting...".to_string());
                    let _ = p.kill();
                    let _ = p.wait();
                }
                return None; // user-cancelled
            }

            if self.is_vpn_connected() {
                if !connected_detected {
                    connected_detected = true;
                    self.connected_at = Some(Instant::now());
                    self.set_status(ConnectionStatus::Connected);
                    self.send_log("Info|Connected.");

                    let kind = if is_reconnect {
                        crate::history::EventKind::Reconnected
                    } else {
                        crate::history::EventKind::Connected
                    };
                    let mut event = crate::history::ConnectionEvent::now(kind);
                    // For reconnects, record how long the previous segment lasted
                    // before it dropped so history shows meaningful durations.
                    event.duration_secs = prev_duration;
                    let _ = crate::history::append_event(&event);
                }
            } else if connected_detected {
                // Tunnel was up and just went down unexpectedly.
                if let Some(ref mut p) = process {
                    let _ = p.kill();
                    let _ = p.wait();
                }
                let duration = self
                    .connected_at
                    .map(|t| t.elapsed().as_secs())
                    .unwrap_or(0);
                return Some(duration); // unexpected disconnect — eligible for reconnect
            } else if start_time.elapsed() > timeout {
                self.set_conn_error("VPN tunnel failed to establish within timeout");
                if let Some(ref mut p) = process {
                    let _ = p.kill();
                }
                return None;
            } else if let Some(ref mut p) = process {
                if !p.is_process_alive() {
                    let reason = p.failure_reason().unwrap_or_else(|| {
                        "OpenConnect process exited before tunnel was established".to_string()
                    });
                    self.set_conn_error(&reason);
                    return None;
                }
            }
            thread::sleep(Duration::from_millis(1000));
        }
    }

    fn cleanup(&self, reconnect_attempts: u32) {
        if let Some(pid) = get_openconnect_pid() {
            let _ = kill_process(pid);
            thread::sleep(Duration::from_millis(500));
        }

        let duration_secs = self.connected_at.map(|t| t.elapsed().as_secs());

        if is_openconnect_running() {
            let err_msg = "Failed to stop OpenConnect. Please close it manually.";
            self.set_conn_error(err_msg);
            let mut event = crate::history::ConnectionEvent::now(crate::history::EventKind::Error);
            event.duration_secs = duration_secs;
            event.message = Some(err_msg.to_string());
            let _ = crate::history::append_event(&event);
        } else if self.connected_at.is_some() {
            // Was connected at some point — record a normal disconnect.
            self.set_status(ConnectionStatus::Disconnected);
            self.send_log("Info|Disconnected.");
            let mut event =
                crate::history::ConnectionEvent::now(crate::history::EventKind::Disconnected);
            event.duration_secs = duration_secs;
            if reconnect_attempts > 0 {
                event.message = Some(format!(
                    "after {} reconnect attempt{}",
                    reconnect_attempts,
                    if reconnect_attempts == 1 { "" } else { "s" }
                ));
            }
            let _ = crate::history::append_event(&event);
        } else if *self.status.lock().expect("session mutex poisoned") == ConnectionStatus::Error {
            // An error occurred before ever connecting (auth failure, tunnel timeout, etc.)
            let mut event = crate::history::ConnectionEvent::now(crate::history::EventKind::Error);
            event.message = self.last_error.lock().expect("session mutex poisoned").clone();
            let _ = crate::history::append_event(&event);
        } else {
            // Never reached the Connected state — record as cancelled, not disconnected.
            self.set_status(ConnectionStatus::Disconnected);
            self.send_log("Info|Cancelled.");
            let event = crate::history::ConnectionEvent::now(crate::history::EventKind::Cancelled);
            let _ = crate::history::append_event(&event);
        }
    }
}

#[derive(Clone)]
pub struct VpnSession {
    config: SessionConfig,
    status: Arc<Mutex<ConnectionStatus>>,
    cancel_token: CancellationToken,
    last_error: Arc<Mutex<Option<String>>>,
    error_category: Arc<Mutex<Option<crate::error::ErrorCategory>>>,
    logs_tx: Arc<Mutex<Option<crossbeam_channel::Sender<String>>>>,
    browser_pid: Arc<Mutex<Option<u32>>>,
}

impl VpnSession {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            status: Arc::new(Mutex::new(ConnectionStatus::Disconnected)),
            cancel_token: CancellationToken::new(),
            last_error: Arc::new(Mutex::new(None)),
            error_category: Arc::new(Mutex::new(None)),
            logs_tx: Arc::new(Mutex::new(None)),
            browser_pid: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_logs_tx(&self, tx: crossbeam_channel::Sender<String>) {
        *self.logs_tx.lock().expect("session mutex poisoned") = Some(tx);
    }

    fn take_browser_pid(&self) -> Option<u32> {
        self.browser_pid.lock().expect("session mutex poisoned").take()
    }

    fn transition_to_disconnecting(&self) {
        let mut status = self.status.lock().expect("session mutex poisoned");
        if matches!(
            *status,
            ConnectionStatus::Connected | ConnectionStatus::Connecting
        ) {
            *status = ConnectionStatus::Disconnecting;
        }
    }

    fn try_begin_connect(&self) -> bool {
        let mut s = self.status.lock().expect("session mutex poisoned");
        if *s != ConnectionStatus::Disconnected && *s != ConnectionStatus::Error {
            return false;
        }
        *s = ConnectionStatus::Connecting;
        *self.last_error.lock().expect("session mutex poisoned") = None;
        *self.error_category.lock().expect("session mutex poisoned") = None;
        true
    }

    pub fn status(&self) -> ConnectionStatus {
        *self.status.lock().expect("session mutex poisoned")
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().expect("session mutex poisoned").clone()
    }

    pub fn error_category(&self) -> Option<crate::error::ErrorCategory> {
        *self.error_category.lock().expect("session mutex poisoned")
    }

    /// Returns true if the session has reached a terminal state.
    pub fn is_finished(&self) -> bool {
        let s = self.status();
        s == ConnectionStatus::Disconnected || s == ConnectionStatus::Error
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
        self.transition_to_disconnecting();

        // Force-kill the browser process to unblock any pending CDP calls.
        // SIGKILL on Unix / taskkill on Windows closes Chrome's socket immediately,
        // unblocking any pending CDP calls without waiting for graceful shutdown.
        if let Some(pid) = self.take_browser_pid() {
            log::info!("Force-killing browser process (PID: {})", pid);
            crate::openconnect::kill_browser_process(pid);
        }
    }

    pub fn connect(&self, provider: Arc<dyn CredentialsProvider>) -> thread::JoinHandle<()> {
        if !self.try_begin_connect() {
            return thread::spawn(|| {});
        }
        let thread = SessionThread::from_session(self);
        thread::spawn(move || thread.run(provider))
    }
}
