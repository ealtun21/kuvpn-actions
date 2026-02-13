use futures::SinkExt;
use iced::{Subscription, Task};
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use tray_icon::{
    menu::{MenuEvent, MenuItem},
    TrayIcon, TrayIconEvent,
};

use crate::config::GuiSettings;
use crate::logger::{GUI_LOGGER, LOGGER_INIT};
use crate::provider::{GuiInteraction, GuiProvider};
use crate::types::{ConnectionStatus, InputRequest, InputRequestWrapper, Message};

pub struct KuVpnGui {
    // Settings
    pub settings: GuiSettings,

    // UI State
    pub show_advanced: bool,
    pub show_console: bool,
    pub logs: Vec<String>,
    pub status: ConnectionStatus,
    pub pending_request: Option<InputRequest>,
    pub current_input: String,
    pub cancel_tx: Option<oneshot::Sender<()>>,
    pub cancel_token: Option<kuvpn::utils::CancellationToken>,
    pub mfa_info: Option<String>,
    pub rotation: f32,
    pub oc_test_result: Option<bool>,
    
    // Tray & Window state
    pub tray_icon: Option<TrayIcon>,
    pub show_item: Option<MenuItem>,
    pub connect_item: Option<MenuItem>,
    pub disconnect_item: Option<MenuItem>,
    pub window_id: Option<iced::window::Id>,
    pub is_visible: bool,
}

impl KuVpnGui {
    pub fn theme(&self, _id: iced::window::Id) -> iced::Theme {
        iced::Theme::Dark
    }

    fn save_settings(&self) {
        if let Err(e) = self.settings.save() {
            log::error!("Failed to save settings: {}", e);
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(id) => {
                log::info!("Window opened with ID: {:?}", id);
                self.window_id = Some(id);
                self.is_visible = true;
                if let Some(item) = &self.show_item {
                    let _ = item.set_text("Toggle Visibility");
                }
                Task::none()
            }
            Message::WindowClosed(id) => {
                log::info!("Window closed with ID: {:?}", id);
                if self.window_id == Some(id) {
                    self.window_id = None;
                    self.is_visible = false;
                }
                Task::none()
            }
            Message::GtkTick => {
                #[cfg(target_os = "linux")]
                {
                    while gtk::events_pending() {
                        gtk::main_iteration();
                    }
                }
                Task::none()
            }
            Message::TrayEvent(event) => {
                match event {
                    TrayIconEvent::Click { .. } => {
                        return self.update(Message::ToggleVisibility {
                            from_close_request: false,
                        });
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::MenuEvent(event) => match event.id.as_ref() {
                "quit" => return iced::exit(),
                "show" => {
                    return self.update(Message::ToggleVisibility {
                        from_close_request: false,
                    });
                }
                "connect" => return self.update(Message::ConnectPressed),
                "disconnect" => return self.update(Message::DisconnectPressed),
                _ => Task::none(),
            },
            Message::CloseToTrayToggled(v) => {
                self.settings.close_to_tray = v;
                self.save_settings();
                Task::none()
            }
            Message::ToggleVisibility { from_close_request } => {
                log::info!(
                    "ToggleVisibility called. visible={}, close_to_tray={}, from_close_request={}",
                    self.is_visible,
                    self.settings.close_to_tray,
                    from_close_request
                );

                if self.is_visible {
                    if from_close_request && !self.settings.close_to_tray {
                        log::info!("Exiting application due to close request");
                        return iced::exit();
                    }
                    log::info!("Closing window to hide");
                    self.is_visible = false;
                    if let Some(id) = self.window_id.take() {
                        return iced::window::close(id);
                    }
                } else {
                    log::info!("Opening window to show");
                    let (id, task) = iced::window::open(iced::window::Settings {
                        exit_on_close_request: false,
                        ..Default::default()
                    });
                    self.window_id = Some(id);
                    return task.map(Message::WindowOpened);
                }
                Task::none()
            }
            Message::UrlChanged(url) => {
                self.settings.url = url;
                self.save_settings();
                Task::none()
            }
            Message::DomainChanged(domain) => {
                self.settings.domain = domain;
                self.save_settings();
                Task::none()
            }
            Message::EscalationToolChanged(tool) => {
                self.settings.escalation_tool = tool;
                self.save_settings();
                Task::none()
            }
            Message::LogLevelSliderChanged(val) => {
                self.settings.log_level_val = val;
                if let Ok(mut guard) = GUI_LOGGER.user_level.lock() {
                    *guard = match val.round() as i32 {
                        0 => log::LevelFilter::Off,
                        1 => log::LevelFilter::Error,
                        2 => log::LevelFilter::Warn,
                        3 => log::LevelFilter::Info,
                        4 => log::LevelFilter::Debug,
                        5 => log::LevelFilter::Trace,
                        _ => log::LevelFilter::Info,
                    };
                }
                self.save_settings();
                Task::none()
            }
            Message::OpenConnectPathChanged(p) => {
                self.settings.openconnect_path = p;
                self.oc_test_result = None;
                self.save_settings();
                Task::none()
            }
            Message::EmailChanged(e) => {
                self.settings.email = e;
                self.save_settings();
                Task::none()
            }
            Message::LoginModeChanged(val) => {
                self.settings.login_mode_val = val;
                self.save_settings();
                Task::none()
            }
            Message::ToggleAdvanced => {
                self.show_advanced = !self.show_advanced;
                Task::none()
            }
            Message::ToggleConsole => {
                self.show_console = !self.show_console;
                Task::none()
            }
            Message::Tick => {
                if self.status == ConnectionStatus::Connecting {
                    self.rotation += 0.1;
                }
                Task::none()
            }
            Message::Watchdog => {
                #[cfg(windows)]
                {
                    let is_running = kuvpn::is_openconnect_running();
                    if self.status == ConnectionStatus::Connecting && is_running {
                        return self.update(Message::StatusChanged(ConnectionStatus::Connected));
                    } else if self.status == ConnectionStatus::Connected && !is_running {
                        return self.update(Message::StatusChanged(ConnectionStatus::Disconnected));
                    }
                }
                Task::none()
            }
            // Clean up required, barely understandable code.
            Message::ConnectPressed => {
                if self.status == ConnectionStatus::Disconnected {
                    // Cancel any orphaned tokens (though status should prevent this, safety first)
                    if let Some(token) = &self.cancel_token {
                        token.cancel();
                    }
                    if let Some(tx) = self.cancel_tx.take() {
                        let _ = tx.send(());
                    }

                    self.status = ConnectionStatus::Connecting;
                    self.logs.clear();
                    self.logs
                        .push("[*] Accessing campus gateway...".to_string());

                    let url = self.settings.url.clone();
                    let domain = self.settings.domain.clone();

                    let (headless, no_auto_login) =
                        match self.settings.login_mode_val.round() as i32 {
                            0 => (true, false),  // Full Automatic
                            1 => (false, false), // Visual Automatic
                            _ => (false, true),  // Manual
                        };

                    let escalation_tool = self.settings.escalation_tool.clone();
                    let openconnect_path = if self.settings.openconnect_path.is_empty() {
                        "openconnect".to_string()
                    } else {
                        self.settings.openconnect_path.clone()
                    };
                    let email = if self.settings.email.is_empty() {
                        None
                    } else {
                        Some(self.settings.email.clone())
                    };

                    let (cancel_tx, mut cancel_rx) = oneshot::channel();
                    self.cancel_tx = Some(cancel_tx);
                    let cancel_token = kuvpn::utils::CancellationToken::new();
                    self.cancel_token = Some(cancel_token.clone());
                    let cancel_token_clone = cancel_token.clone();

                    return Task::stream(iced::stream::channel(
                        100,
                        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                            let (log_tx, mut log_rx) = mpsc::channel(100);
                            let (interaction_tx, mut interaction_rx) = mpsc::channel(10);
                            let (child_tx, mut child_rx) = mpsc::channel::<u32>(1);

                            if let Ok(mut guard) = GUI_LOGGER.tx.lock() {
                                *guard = Some(log_tx.clone());
                            }

                            LOGGER_INIT.call_once(|| {
                                let _ = log::set_logger(&GUI_LOGGER);
                                log::set_max_level(log::LevelFilter::Trace);
                            });

                            let url_c = url.clone();
                            let domain_c = domain.clone();
                            let log_tx_c = log_tx.clone();
                            let cancel_token_thread = cancel_token_clone.clone();

                            std::thread::spawn(move || {
                                let provider = GuiProvider {
                                    interaction_tx,
                                    cancel_token: cancel_token_thread,
                                };
                                let dsid_res = kuvpn::run_login_and_get_dsid(
                                    headless,
                                    &url_c,
                                    &domain_c,
                                    "Mozilla/5.0",
                                    no_auto_login,
                                    email,
                                    &provider,
                                    Some(cancel_token_clone),
                                );

                                let dsid = match dsid_res {
                                    Ok(d) => d,
                                    Err(e) => {
                                        let _ = log_tx_c.blocking_send(format!("Error|{}", e));
                                        return;
                                    }
                                };

                                let _ = log_tx_c.blocking_send(
                                    "Info|Initializing Koç University tunnel...".to_string(),
                                );
                                let final_oc_path =
                                    match kuvpn::locate_openconnect(&openconnect_path) {
                                        Some(p) => p,
                                        None => {
                                            let _ = log_tx_c.blocking_send(format!(
                                                "Error|Could not locate '{}'",
                                                openconnect_path
                                            ));
                                            return;
                                        }
                                    };

                                match kuvpn::execute_openconnect(
                                    dsid,
                                    url_c,
                                    &Some(escalation_tool),
                                    &final_oc_path,
                                    Stdio::piped(),
                                    Stdio::piped(),
                                ) {
                                    Ok(mut child) => {
                                        let stdout = child.stdout.take().unwrap();
                                        let stderr = child.stderr.take().unwrap();
                                        let pid = child.id();

                                        let _ = child_tx.blocking_send(pid);

                                        let log_tx_stdout = log_tx_c.clone();
                                        std::thread::spawn(move || {
                                            let reader = BufReader::new(stdout);
                                            for line in reader.lines() {
                                                if let Ok(l) = line {
                                                    let _ = log_tx_stdout
                                                        .blocking_send(format!("Info|{}", l));
                                                }
                                            }
                                        });

                                        let log_tx_stderr = log_tx_c.clone();
                                        std::thread::spawn(move || {
                                            let reader = BufReader::new(stderr);
                                            for line in reader.lines() {
                                                if let Ok(l) = line {
                                                    let _ = log_tx_stderr
                                                        .blocking_send(format!("Warn|{}", l));
                                                }
                                            }
                                        });

                                        let _ = child.wait();
                                    }
                                    Err(e) => {
                                        let _ = log_tx_c.blocking_send(format!("Error|{}", e));
                                    }
                                }
                            });

                            let mut active_pid: Option<u32> = None;

                            loop {
                                tokio::select! {
                                    res = log_rx.recv() => {
                                        // TODO, use connection watcher and do not use log based anything, anywhere.
                                        if let Some(log) = res {
                                            if log.contains("Established") ||
                                               log.contains("Connected as") ||
                                               log.contains("Connected to HTTPS") ||
                                               log.contains("Session established") {
                                                let _ = output.send(Message::StatusChanged(ConnectionStatus::Connected)).await;
                                            }
                                            let _ = output.send(Message::LogAppended(log)).await;
                                        } else {
                                            break;
                                        }
                                    }
                                    res = interaction_rx.recv() => {
                                        match res {
                                            Some(GuiInteraction::Request(req)) => {
                                                let _ = output.send(Message::RequestInput(Arc::new(InputRequestWrapper(Mutex::new(Some(req)))))).await;
                                            }
                                            Some(GuiInteraction::MfaPush(code)) => {
                                                let _ = output.send(Message::MfaPushReceived(code)).await;
                                            }
                                            Some(GuiInteraction::MfaComplete) => {
                                                let _ = output.send(Message::MfaCompleteReceived).await;
                                            }
                                            None => break,
                                        }
                                    }
                                    child_res = child_rx.recv() => {
                                        active_pid = child_res;
                                    }
                                    _ = &mut cancel_rx => {
                                        cancel_token.cancel();
                                        if let Some(pid) = active_pid.take() {
                                            let _ = kuvpn::kill_process(pid);
                                        }
                                        break;
                                    }
                                }
                            }
                            let _ = output.send(Message::ConnectionFinished(None)).await;
                        },
                    ));
                }
                Task::none()
            }
            Message::DisconnectPressed => {
                if let Some(token) = self.cancel_token.take() {
                    token.cancel();
                }
                if let Some(tx) = self.cancel_tx.take() {
                    let _ = tx.send(());
                }
                Task::none()
            }
            // TODO: Clean up, causes lag, scrolling does not follow last messege, can not copy text.
            Message::LogAppended(raw_log) => {
                let parts: Vec<&str> = raw_log.splitn(2, '|').collect();
                if parts.len() < 2 {
                    return Task::none();
                }
                let lvl_str = parts[0];
                let log_msg = parts[1];

                let record_level = match lvl_str {
                    "Error" => log::Level::Error,
                    "Warn" => log::Level::Warn,
                    "Info" => log::Level::Info,
                    "Debug" => log::Level::Debug,
                    "Trace" => log::Level::Trace,
                    _ => log::Level::Info,
                };

                // User visibility filtering
                let user_filter = if let Ok(guard) = GUI_LOGGER.user_level.lock() {
                    *guard
                } else {
                    log::LevelFilter::Info
                };

                if record_level <= user_filter {
                    self.logs.push(format!("[*] {}", log_msg));
                    if self.logs.len() > 500 {
                        self.logs.remove(0);
                    }
                }
                Task::none()
            }
            Message::MfaPushReceived(code) => {
                self.mfa_info = Some(code);
                if !self.is_visible {
                    return self.update(Message::ToggleVisibility {
                        from_close_request: false,
                    });
                }
                Task::none()
            }
            Message::MfaCompleteReceived => {
                self.mfa_info = None;
                Task::none()
            }
            Message::RequestInput(wrapper) => {
                if let Ok(mut guard) = wrapper.0.lock() {
                    if let Some(req) = guard.take() {
                        self.pending_request = Some(req);
                        self.current_input = String::new();
                        if !self.is_visible {
                            return self.update(Message::ToggleVisibility {
                                from_close_request: false,
                            });
                        }
                    }
                }
                Task::none()
            }
            Message::InputChanged(val) => {
                self.current_input = val;
                Task::none()
            }
            Message::SubmitInput => {
                if let Some(req) = self.pending_request.take() {
                    let _ = req.response_tx.send(self.current_input.clone());
                    self.current_input = String::new();
                }
                Task::none()
            }
            Message::ClearSessionPressed => {
                match kuvpn::get_user_data_dir() {
                    Ok(dir) => {
                        if dir.exists() {
                            if let Err(e) = std::fs::remove_dir_all(&dir) {
                                self.logs
                                    .push(format!("[!] Failed to clear session: {}", e));
                            } else {
                                self.logs.push("[✓] Saved session data wiped.".to_string());
                            }
                        } else {
                            self.logs.push("[*] No active session found.".to_string());
                        }
                    }
                    Err(e) => {
                        self.logs.push(format!("[!] System error: {}", e));
                    }
                }
                Task::none()
            }
            Message::ConnectionFinished(err) => {
                self.status = ConnectionStatus::Disconnected;
                self.mfa_info = None;
                if let Some(e) = err {
                    self.logs.push(format!("[!] Session Error: {}", e));
                }
                Task::none()
            }
            Message::StatusChanged(status) => {
                if self.status != status {
                    self.status = status;
                    if let Some(item) = &self.connect_item {
                        let _ = item.set_enabled(status == ConnectionStatus::Disconnected);
                    }
                    if let Some(item) = &self.disconnect_item {
                        let _ = item.set_enabled(status == ConnectionStatus::Connected);
                    }
                }
                Task::none()
            }
            Message::ResetSettings => {
                self.settings = GuiSettings::default();
                self.save_settings();
                self.oc_test_result = None;
                // We also need to update the log level filter immediately
                if let Ok(mut guard) = GUI_LOGGER.user_level.lock() {
                    *guard = log::LevelFilter::Error; // Default
                }
                Task::none()
            }
            Message::TestOpenConnect => {
                let path = self.settings.openconnect_path.clone();
                Task::perform(
                    async move { kuvpn::locate_openconnect(&path).is_some() },
                    Message::OpenConnectTestResult,
                )
            }
            Message::OpenConnectTestResult(success) => {
                self.oc_test_result = Some(success);
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![];

        if self.status == ConnectionStatus::Connecting {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
            );
        }

        subs.push(iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Watchdog));

        // GTK Event Loop pump (for Tray Icon on Linux)
        #[cfg(target_os = "linux")]
        subs.push(
            iced::time::every(std::time::Duration::from_millis(20)).map(|_| Message::GtkTick),
        );

        // Window events
        subs.push(
            iced::window::close_requests().map(|_| Message::ToggleVisibility {
                from_close_request: true,
            }),
        );
        subs.push(iced::window::close_events().map(Message::WindowClosed));

        // Tray & Menu events
        subs.push(Subscription::run(|| {
            iced::stream::channel(
                10,
                |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                    let tray_rx = TrayIconEvent::receiver();
                    let menu_rx = MenuEvent::receiver();
                    loop {
                        if let Ok(event) = tray_rx.try_recv() {
                            let _ = output.send(Message::TrayEvent(event)).await;
                        }
                        if let Ok(event) = menu_rx.try_recv() {
                            let _ = output.send(Message::MenuEvent(event)).await;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                },
            )
        }));

        Subscription::batch(subs)
    }
}

impl Default for KuVpnGui {
    fn default() -> Self {
        let settings = GuiSettings::load();

        // Ensure logger is synced with loaded settings
        if let Ok(mut guard) = GUI_LOGGER.user_level.lock() {
            *guard = match settings.log_level_val.round() as i32 {
                0 => log::LevelFilter::Off,
                1 => log::LevelFilter::Error,
                2 => log::LevelFilter::Warn,
                3 => log::LevelFilter::Info,
                4 => log::LevelFilter::Debug,
                5 => log::LevelFilter::Trace,
                _ => log::LevelFilter::Info,
            };
        }

        Self {
            settings,
            show_advanced: false,
            show_console: false,
            logs: vec!["Ready for secure campus access.".to_string()],
            status: ConnectionStatus::Disconnected,
            pending_request: None,
            current_input: String::new(),
            cancel_tx: None,
            cancel_token: None,
            mfa_info: None,
            rotation: 0.0,
            oc_test_result: None,
            tray_icon: None,
            show_item: None,
            connect_item: None,
            disconnect_item: None,
            window_id: None,
            is_visible: false,
        }
    }
}
