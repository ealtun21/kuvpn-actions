use crate::utils::{CancellationToken, CredentialsProvider};
use crate::openconnect::{locate_openconnect, execute_openconnect, is_openconnect_running, VpnProcess};
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

pub struct SessionConfig {
    pub url: String,
    pub domain: String,
    pub user_agent: String,
    pub headless: bool,
    pub no_auto_login: bool,
    pub email: Option<String>,
    pub openconnect_path: String,
    pub escalation_tool: Option<String>,
}

pub struct VpnSession {
    config: SessionConfig,
    status: Arc<Mutex<ConnectionStatus>>,
    cancel_token: CancellationToken,
    last_error: Arc<Mutex<Option<String>>>,
    logs_tx: Arc<Mutex<Option<crossbeam_channel::Sender<String>>>>,
}

impl VpnSession {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            status: Arc::new(Mutex::new(ConnectionStatus::Disconnected)),
            cancel_token: CancellationToken::new(),
            last_error: Arc::new(Mutex::new(None)),
            logs_tx: Arc::new(Mutex::new(None)),
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

    pub fn cancel(&self) {
        self.cancel_token.cancel();
        let mut status = self.status.lock().unwrap();
        if *status == ConnectionStatus::Connected || *status == ConnectionStatus::Connecting {
            *status = ConnectionStatus::Disconnecting;
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
        let config = SessionConfig {
            url: self.config.url.clone(),
            domain: self.config.domain.clone(),
            user_agent: self.config.user_agent.clone(),
            headless: self.config.headless,
            no_auto_login: self.config.no_auto_login,
            email: self.config.email.clone(),
            openconnect_path: self.config.openconnect_path.clone(),
            escalation_tool: self.config.escalation_tool.clone(),
        };
        let cancel_token = self.cancel_token.clone();
        let last_error = Arc::clone(&self.last_error);
        let logs_tx = Arc::clone(&self.logs_tx);

        thread::spawn(move || {
            let log = |msg: String| {
                if let Some(tx) = logs_tx.lock().unwrap().as_ref() {
                    let _ = tx.send(msg);
                }
            };

            // Smart check: don't start if already running
            if is_openconnect_running() {
                log("Info|OpenConnect is already running.".to_string());
                *status.lock().unwrap() = ConnectionStatus::Connected;
                return;
            }

            log("Info|Accessing campus gateway...".to_string());

            // 1. Get DSID
            let dsid = match run_login_and_get_dsid(
                config.headless,
                &config.url,
                &config.domain,
                &config.user_agent,
                config.no_auto_login,
                config.email,
                provider.as_ref(),
                Some(cancel_token.clone()),
            ) {
                Ok(d) => d,
                Err(e) => {
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
                    *last_error.lock().unwrap() = Some(format!("Could not locate openconnect at {}", config.openconnect_path));
                    log(format!("Error|Could not locate openconnect at {}", config.openconnect_path));
                    return;
                }
            };

            // 3. Execute OpenConnect
            let mut process = match execute_openconnect(
                dsid,
                config.url,
                &config.escalation_tool,
                &oc_path,
                Stdio::piped(),
                Stdio::piped(),
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
            if let VpnProcess::Unix(ref mut child) = process {
                let stdout = child.stdout.take().unwrap();
                let stderr = child.stderr.take().unwrap();
                
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

            // 4. Watchdog loop
            let start_time = Instant::now();
            let mut connected_detected = false;

            loop {
                if cancel_token.is_cancelled() {
                    break;
                }

                let is_running = is_openconnect_running();
                
                if is_running {
                    if !connected_detected {
                        connected_detected = true;
                        *status.lock().unwrap() = ConnectionStatus::Connected;
                        log("Info|Connected.".to_string());
                    }
                } else if connected_detected {
                    // We were connected, but now the process is gone
                    break;
                } else if start_time.elapsed() > Duration::from_secs(15) {
                    // We never managed to start the process within 15 seconds
                    let mut s = status.lock().unwrap();
                    *s = ConnectionStatus::Error;
                    *last_error.lock().unwrap() = Some("OpenConnect failed to start or exited early".to_string());
                    log("Error|OpenConnect failed to start or exited early".to_string());
                    return;
                }

                thread::sleep(Duration::from_millis(1000));
            }

            // Cleanup
            log("Info|Disconnecting...".to_string());
            let _ = process.kill();
            let _ = process.wait();

            *status.lock().unwrap() = ConnectionStatus::Disconnected;
            log("Info|Disconnected.".to_string());
        })
    }
}
