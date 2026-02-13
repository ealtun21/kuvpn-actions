use crate::utils::{CancellationToken, CredentialsProvider};
use crate::openconnect::{locate_openconnect, execute_openconnect, is_openconnect_running, is_vpn_interface_up, VpnProcess};
use crate::dsid::run_login_and_get_dsid;
use std::process::Stdio;
use std::io::{BufRead, BufReader};
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

pub struct VpnSession {
    config: SessionConfig,
    status: Arc<Mutex<ConnectionStatus>>,
    cancel_token: CancellationToken,
    last_error: Arc<Mutex<Option<String>>>,
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
            logs_tx: Arc::new(Mutex::new(None)),
            browser_pid: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_logs_tx(&self, tx: crossbeam_channel::Sender<String>) {
        *self.logs_tx.lock().unwrap() = Some(tx);
    }

    pub fn status(&self) -> ConnectionStatus {
        *self.status.lock().unwrap()
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().clone()
    }

    /// Returns true if the session has reached a terminal state.
    pub fn is_finished(&self) -> bool {
        let s = self.status();
        s == ConnectionStatus::Disconnected || s == ConnectionStatus::Error
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
        let mut status = self.status.lock().unwrap();
        if *status == ConnectionStatus::Connected || *status == ConnectionStatus::Connecting {
            *status = ConnectionStatus::Disconnecting;
        }
        drop(status);

        // Force-kill the browser process to unblock any pending CDP calls
        if let Some(pid) = self.browser_pid.lock().unwrap().take() {
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
        {
            let mut s = self.status.lock().unwrap();
            if *s != ConnectionStatus::Disconnected && *s != ConnectionStatus::Error {
                return thread::spawn(|| {});
            }
            *s = ConnectionStatus::Connecting;
            *self.last_error.lock().unwrap() = None;
        }

        let status = Arc::clone(&self.status);
        let config = self.config.clone();
        let cancel_token = self.cancel_token.clone();
        let last_error = Arc::clone(&self.last_error);
        let logs_tx = Arc::clone(&self.logs_tx);
        let browser_pid = Arc::clone(&self.browser_pid);

        thread::spawn(move || {
            let log = |msg: String| {
                if let Some(tx) = logs_tx.lock().unwrap().as_ref() {
                    let _ = tx.send(msg);
                }
            };

            let interface_name = &config.interface_name;

            // Check if VPN is already connected.
            // On Unix, we use the named TUN interface (precise).
            // On Windows, we fall back to process-name detection since
            // --interface is not passed on Windows.
            let is_vpn_connected = || -> bool {
                is_vpn_interface_up(interface_name) || is_openconnect_running()
            };

            // Check if already connected
            let already_connected = is_vpn_connected();
            let mut process: Option<VpnProcess> = None;

            if already_connected {
                log("Info|VPN interface already active, monitoring...".to_string());
                *status.lock().unwrap() = ConnectionStatus::Connected;
            } else {
                log("Info|Accessing campus gateway...".to_string());

                // 1. Get DSID
                let dsid = match run_login_and_get_dsid(
                    config.headless,
                    &config.url,
                    &config.domain,
                    &config.user_agent,
                    config.no_auto_login,
                    config.email.clone(),
                    provider.as_ref(),
                    Some(cancel_token.clone()),
                    Some(Arc::clone(&browser_pid)),
                ) {
                    Ok(d) => {
                        // Browser is done, clear PID so cancel() won't kill a dead process
                        *browser_pid.lock().unwrap() = None;
                        d
                    }
                    Err(e) => {
                        *browser_pid.lock().unwrap() = None;
                        let mut s = status.lock().unwrap();
                        if *s != ConnectionStatus::Disconnecting {
                            *s = ConnectionStatus::Error;
                            *last_error.lock().unwrap() = Some(e.to_string());
                            log(format!("Error|{}", e));
                        } else {
                            *s = ConnectionStatus::Disconnected;
                        }
                        return;
                    }
                };

                if cancel_token.is_cancelled() {
                    *status.lock().unwrap() = ConnectionStatus::Disconnected;
                    return;
                }

                log("Info|Initializing tunnel...".to_string());

                // 2. Locate OpenConnect
                let oc_path = match locate_openconnect(&config.openconnect_path) {
                    Some(p) => p,
                    None => {
                        *status.lock().unwrap() = ConnectionStatus::Error;
                        *last_error.lock().unwrap() = Some(format!(
                            "Could not locate openconnect at '{}'. Please install openconnect or set the correct path.",
                            config.openconnect_path
                        ));
                        log(format!("Error|Could not locate openconnect at '{}'", config.openconnect_path));
                        return;
                    }
                };

                // 3. Execute OpenConnect
                let mut proc = match execute_openconnect(
                    dsid,
                    config.url.clone(),
                    &config.escalation_tool,
                    &oc_path,
                    Stdio::piped(),
                    Stdio::piped(),
                    interface_name,
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        *status.lock().unwrap() = ConnectionStatus::Error;
                        *last_error.lock().unwrap() = Some(e.to_string());
                        log(format!("Error|{}", e));
                        return;
                    }
                };

                // Capture logs if possible (Unix)
                if let VpnProcess::Unix(ref mut child) = proc {
                    if let Some(stdout) = child.stdout.take() {
                        let logs_tx_stdout = Arc::clone(&logs_tx);
                        thread::spawn(move || {
                            let reader = BufReader::new(stdout);
                            for line in reader.lines() {
                                if let Ok(l) = line {
                                    if let Some(tx) = logs_tx_stdout.lock().unwrap().as_ref() {
                                        let _ = tx.send(format!("Info|{}", l));
                                    }
                                }
                            }
                        });
                    }

                    if let Some(stderr) = child.stderr.take() {
                        let logs_tx_stderr = Arc::clone(&logs_tx);
                        thread::spawn(move || {
                            let reader = BufReader::new(stderr);
                            for line in reader.lines() {
                                if let Ok(l) = line {
                                    if let Some(tx) = logs_tx_stderr.lock().unwrap().as_ref() {
                                        let _ = tx.send(format!("Warn|{}", l));
                                    }
                                }
                            }
                        });
                    }
                }

                process = Some(proc);
            }

            // 4. Watchdog loop - uses TUN interface detection as primary signal
            let start_time = Instant::now();
            let mut connected_detected = already_connected;
            let timeout = Duration::from_secs(30);

            loop {
                if cancel_token.is_cancelled() {
                    break;
                }

                let interface_up = is_vpn_connected();

                if interface_up {
                    if !connected_detected {
                        connected_detected = true;
                        *status.lock().unwrap() = ConnectionStatus::Connected;
                        log("Info|Connected.".to_string());
                    }
                } else if connected_detected {
                    // Interface went down - disconnected
                    break;
                } else if start_time.elapsed() > timeout {
                    *status.lock().unwrap() = ConnectionStatus::Error;
                    *last_error.lock().unwrap() = Some(
                        "VPN tunnel failed to establish within timeout".to_string()
                    );
                    log("Error|VPN tunnel failed to establish within timeout".to_string());
                    if let Some(ref mut p) = process { let _ = p.kill(); }
                    return;
                } else if let Some(ref mut p) = process {
                    // Check if our spawned process (sudo/doas/pkexec) is still alive.
                    // This is the key check: if sudo is waiting for a password, the process
                    // is still alive and we should keep waiting. Only fail if the process
                    // has actually exited (e.g. wrong password, permission denied).
                    if !p.is_process_alive() {
                        *status.lock().unwrap() = ConnectionStatus::Error;
                        *last_error.lock().unwrap() = Some(
                            "OpenConnect process exited before tunnel was established".to_string()
                        );
                        log("Error|OpenConnect process exited before tunnel was established".to_string());
                        return;
                    }
                }

                thread::sleep(Duration::from_millis(1000));
            }

            // Cleanup
            log("Info|Disconnecting...".to_string());
            if let Some(ref mut p) = process {
                let _ = p.kill();
                let _ = p.wait();
            } else {
                // We don't own the process (already-connected path).
                // Use PID-based kill as fallback.
                use crate::openconnect::{get_openconnect_pid, kill_process};
                if let Some(pid) = get_openconnect_pid() {
                    log("Info|Sending stop signal to openconnect...".to_string());
                    let _ = kill_process(pid);
                }
            }

            *status.lock().unwrap() = ConnectionStatus::Disconnected;
            log("Info|Disconnected.".to_string());
        })
    }
}
