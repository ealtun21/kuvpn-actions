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
            if let Some(tx) = logs_tx.lock().unwrap().as_ref() {
                let _ = tx.send(format!("{}|{}", level, line));
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
        }
    }

    fn send_log(&self, msg: impl Into<String>) {
        if let Some(tx) = self.logs_tx.lock().unwrap().as_ref() {
            let _ = tx.send(msg.into());
        }
    }

    fn set_status(&self, s: ConnectionStatus) {
        *self.status.lock().unwrap() = s;
    }

    fn set_conn_error(&self, msg: &str) {
        *self.status.lock().unwrap() = ConnectionStatus::Error;
        *self.last_error.lock().unwrap() = Some(msg.to_string());
        *self.error_category.lock().unwrap() = Some(crate::error::ErrorCategory::Connection);
        self.send_log(format!("Error|{}", msg));
    }

    fn clear_browser_pid(&self) {
        *self.browser_pid.lock().unwrap() = None;
    }

    fn handle_login_error(&self, e: anyhow::Error) {
        let mut status = self.status.lock().unwrap();
        if *status == ConnectionStatus::Disconnecting {
            *status = ConnectionStatus::Disconnected;
        } else {
            let category = e
                .downcast_ref::<crate::error::AuthError>()
                .map(|ae| ae.category())
                .unwrap_or(crate::error::ErrorCategory::Authentication);
            *status = ConnectionStatus::Error;
            *self.last_error.lock().unwrap() = Some(e.to_string());
            *self.error_category.lock().unwrap() = Some(category);
            self.send_log(format!("Error|{}", e));
        }
    }

    fn run(self, provider: Arc<dyn CredentialsProvider>) {
        if self.is_vpn_connected() {
            self.send_log("Info|VPN interface already active, monitoring...");
            self.set_status(ConnectionStatus::Connected);
            self.run_watchdog(None);
        } else {
            match self.launch_vpn(&provider) {
                Ok(Some(proc)) => self.run_watchdog(Some(proc)),
                Ok(None) => {} // cancelled
                Err(_) => return,
            }
        }
        self.cleanup();
    }

    fn is_vpn_connected(&self) -> bool {
        is_vpn_interface_up(&self.config.interface_name) || is_openconnect_running()
    }

    fn launch_vpn(
        &self,
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
        result.map_err(|e| self.handle_login_error(e))
    }

    fn start_openconnect(
        &self,
        dsid: String,
        #[cfg_attr(not(unix), allow(unused_variables))] provider: &Arc<dyn CredentialsProvider>,
    ) -> Result<VpnProcess, ()> {
        let runner = OpenConnectRunner::locate(
            &self.config.openconnect_path,
            self.config.interface_name.to_string(),
            self.config.escalation_tool.clone(),
        )
        .ok_or_else(|| {
            self.set_conn_error(&format!(
                "Could not locate openconnect at '{}'. Please install openconnect or set the correct path.",
                self.config.openconnect_path
            ));
        })?;

        let sudo_password = {
            #[cfg(unix)]
            {
                let log_fn = |m| self.send_log(m);
                resolve_sudo_password(
                    &self.config.escalation_tool,
                    provider.as_ref(),
                    &self.cancel_token,
                    &log_fn,
                )
                .map_err(|_| self.set_status(ConnectionStatus::Disconnected))?
            }
            #[cfg(not(unix))]
            {
                None
            }
        };

        runner
            .execute(
                dsid,
                self.config.url.clone(),
                Stdio::piped(),
                Stdio::piped(),
                sudo_password,
            )
            .map_err(|e| self.set_conn_error(&e.to_string()))
    }

    fn spawn_log_readers(&self, proc: &mut VpnProcess) {
        if let VpnProcess::Unix(ref mut child) = proc {
            if let Some(stdout) = child.stdout.take() {
                spawn_stream_reader(stdout, "Info", Arc::clone(&self.logs_tx));
            }
            if let Some(stderr) = child.stderr.take() {
                spawn_stream_reader(stderr, "Warn", Arc::clone(&self.logs_tx));
            }
        }
    }

    fn run_watchdog(&self, mut process: Option<VpnProcess>) {
        let start_time = Instant::now();
        let mut connected_detected = process.is_none();
        let timeout = Duration::from_secs(30);

        loop {
            if self.cancel_token.is_cancelled() {
                break;
            }

            if self.is_vpn_connected() {
                if !connected_detected {
                    connected_detected = true;
                    self.set_status(ConnectionStatus::Connected);
                    self.send_log("Info|Connected.");
                }
            } else if connected_detected {
                break;
            } else if start_time.elapsed() > timeout {
                self.set_conn_error("VPN tunnel failed to establish within timeout");
                if let Some(ref mut p) = process {
                    let _ = p.kill();
                }
                return;
            } else if let Some(ref mut p) = process {
                if !p.is_process_alive() {
                    self.set_conn_error("OpenConnect process exited before tunnel was established");
                    return;
                }
            }
            thread::sleep(Duration::from_millis(1000));
        }

        if let Some(ref mut p) = process {
            self.send_log("Info|Disconnecting...".to_string());
            let _ = p.kill();
            let _ = p.wait();
        }
    }

    fn cleanup(&self) {
        if let Some(pid) = get_openconnect_pid() {
            let _ = kill_process(pid);
            thread::sleep(Duration::from_millis(500));
        }

        if is_openconnect_running() {
            self.set_conn_error("Failed to stop OpenConnect. Please close it manually.");
        } else {
            self.set_status(ConnectionStatus::Disconnected);
            self.send_log("Info|Disconnected.");
        }
    }
}

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
        *self.logs_tx.lock().unwrap() = Some(tx);
    }

    fn take_browser_pid(&self) -> Option<u32> {
        self.browser_pid.lock().unwrap().take()
    }

    fn transition_to_disconnecting(&self) {
        let mut status = self.status.lock().unwrap();
        if matches!(
            *status,
            ConnectionStatus::Connected | ConnectionStatus::Connecting
        ) {
            *status = ConnectionStatus::Disconnecting;
        }
    }

    fn try_begin_connect(&self) -> bool {
        let mut s = self.status.lock().unwrap();
        if *s != ConnectionStatus::Disconnected && *s != ConnectionStatus::Error {
            return false;
        }
        *s = ConnectionStatus::Connecting;
        *self.last_error.lock().unwrap() = None;
        *self.error_category.lock().unwrap() = None;
        true
    }

    pub fn status(&self) -> ConnectionStatus {
        *self.status.lock().unwrap()
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().clone()
    }

    pub fn error_category(&self) -> Option<crate::error::ErrorCategory> {
        *self.error_category.lock().unwrap()
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

        // Force-kill the browser process to unblock any pending CDP calls
        if let Some(pid) = self.take_browser_pid() {
            log::info!("[*] Force-killing browser process (PID: {})", pid);
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;
                let _ = signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                let _ = std::process::Command::new("taskkill")
                    .creation_flags(CREATE_NO_WINDOW)
                    .args(["/F", "/PID", &pid.to_string()])
                    .status();
            }
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
