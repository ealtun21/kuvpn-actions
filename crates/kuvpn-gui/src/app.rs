use futures::SinkExt;
use iced::{Subscription, Task};
use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{MenuEvent, MenuItem},
    TrayIcon, TrayIconEvent,
};

use crate::config::GuiSettings;
use crate::provider::{GuiInteraction, GuiProvider};
use crate::types::{ConnectionStatus, InputRequest, InputRequestWrapper, Message, log_level_from_slider, login_mode_flags};
use kuvpn::{VpnSession, SessionConfig};

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
    pub mfa_info: Option<String>,
    pub rotation: f32,
    pub oc_test_result: Option<bool>,
    
    // VPN Session
    pub session: Option<Arc<VpnSession>>,

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
                if let Ok(mut guard) = crate::logger::GUI_LOGGER.user_level.lock() {
                    *guard = log_level_from_slider(val);
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
                if self.status == ConnectionStatus::Connecting || self.status == ConnectionStatus::Disconnecting {
                    self.rotation += 0.1;
                }
                Task::none()
            }
            // ConnectPressed handled above
            Message::ConnectPressed => {
                if self.status == ConnectionStatus::Disconnected || self.status == ConnectionStatus::Error {
                    let (headless, no_auto_login) = login_mode_flags(self.settings.login_mode_val);

                    let config = SessionConfig {
                        url: self.settings.url.clone(),
                        domain: self.settings.domain.clone(),
                        user_agent: "Mozilla/5.0".to_string(),
                        headless,
                        no_auto_login,
                        email: if self.settings.email.is_empty() { None } else { Some(self.settings.email.clone()) },
                        openconnect_path: if self.settings.openconnect_path.is_empty() { "openconnect".to_string() } else { self.settings.openconnect_path.clone() },
                        escalation_tool: Some(self.settings.escalation_tool.clone()),
                        interface_name: "kuvpn0".to_string(),
                    };

                    let session = Arc::new(VpnSession::new(config));
                    self.session = Some(Arc::clone(&session));
                    self.status = ConnectionStatus::Connecting;
                    self.logs.clear();

                    let (log_tx, log_rx) = crossbeam_channel::unbounded();
                    session.set_logs_tx(log_tx.clone());

                    // Initialize global logger once
                    crate::logger::LOGGER_INIT.call_once(|| {
                        let _ = log::set_logger(&crate::logger::GUI_LOGGER);
                        log::set_max_level(log::LevelFilter::Trace);
                    });

                    // Bridge global logger to our log stream
                    let (gui_tx, mut gui_rx) = tokio::sync::mpsc::channel(100);
                    if let Ok(mut guard) = crate::logger::GUI_LOGGER.tx.lock() {
                        *guard = Some(gui_tx);
                    }

                    return Task::stream(iced::stream::channel(
                        100,
                        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                            use futures::SinkExt;
                            let (interaction_tx, mut interaction_rx) = tokio::sync::mpsc::channel(10);
                            
                            let provider = Arc::new(GuiProvider {
                                interaction_tx,
                                cancel_token: kuvpn::utils::CancellationToken::new(), 
                            });

                            let session_c = Arc::clone(&session);
                            let _join_handle = session.connect(provider);

                            loop {
                                // Poll logs from session
                                while let Ok(log_msg) = log_rx.try_recv() {
                                    let _ = output.send(Message::LogAppended(log_msg)).await;
                                }

                                // Poll logs from global logger
                                while let Ok(log_msg) = gui_rx.try_recv() {
                                    let _ = output.send(Message::LogAppended(log_msg)).await;
                                }

                                // Poll status
                                let current_status = session_c.status();
                                let _ = output.send(Message::StatusChanged(current_status)).await;

                                if current_status == ConnectionStatus::Disconnected || current_status == ConnectionStatus::Error {
                                    break;
                                }

                                // Poll interactions
                                match interaction_rx.try_recv() {
                                    Ok(GuiInteraction::Request(req)) => {
                                        let _ = output.send(Message::RequestInput(Arc::new(InputRequestWrapper(Mutex::new(Some(req)))))).await;
                                    }
                                    Ok(GuiInteraction::MfaPush(code)) => {
                                        let _ = output.send(Message::MfaPushReceived(code)).await;
                                    }
                                    Ok(GuiInteraction::MfaComplete) => {
                                        let _ = output.send(Message::MfaCompleteReceived).await;
                                    }
                                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
                                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
                                }

                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            }
                            let _ = output.send(Message::ConnectionFinished(session_c.last_error())).await;
                        },
                    ));
                }
                Task::none()
            }
            Message::DisconnectPressed => {
                if let Some(session) = &self.session {
                    session.cancel();
                }
                Task::none()
            }
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
            
                            let user_filter = if let Ok(guard) = crate::logger::GUI_LOGGER.user_level.lock() {
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

        if self.status == ConnectionStatus::Connecting || self.status == ConnectionStatus::Disconnecting {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick),
            );
        }

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

        if let Ok(mut guard) = crate::logger::GUI_LOGGER.user_level.lock() {
            *guard = log_level_from_slider(settings.log_level_val);
        }

        Self {
            settings,
            show_advanced: false,
            show_console: false,
            logs: vec!["Ready for secure campus access.".to_string()],
            status: ConnectionStatus::Disconnected,
            pending_request: None,
            current_input: String::new(),
            mfa_info: None,
            rotation: 0.0,
            oc_test_result: None,
            session: None,
            tray_icon: None,
            show_item: None,
            connect_item: None,
            disconnect_item: None,
            window_id: None,
            is_visible: false,
        }
    }
}
