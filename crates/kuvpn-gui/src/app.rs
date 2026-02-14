use futures::SinkExt;
use iced::{Subscription, Task};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tray_icon::{
    menu::{MenuEvent, MenuItem},
    TrayIcon, TrayIconEvent,
};

use crate::config::GuiSettings;
use crate::provider::{GuiInteraction, GuiProvider};
use crate::types::{
    log_level_from_slider, login_mode_flags, ConnectionStatus, InputRequest, InputRequestWrapper,
    Message, Tab,
};
use kuvpn::{SessionConfig, VpnSession};

pub struct KuVpnGui {
    // Settings
    pub settings: GuiSettings,

    // UI State
    pub current_tab: Tab,
    pub logs: Vec<String>,
    pub status: ConnectionStatus,
    pub pending_request: Option<InputRequest>,
    pub current_input: String,
    pub mfa_info: Option<String>,
    pub status_message: String,
    pub error_message: Option<String>,
    pub rotation: f32,
    pub oc_test_result: Option<bool>,
    pub automation_warning: Option<String>,

    // VPN Session
    pub session: Option<Arc<VpnSession>>,

    // Tray & Window state
    pub tray_icon: Option<TrayIcon>,
    pub show_item: Option<MenuItem>,
    pub connect_item: Option<MenuItem>,
    pub disconnect_item: Option<MenuItem>,
    pub window_id: Option<iced::window::Id>,
    pub is_visible: bool,
    pub window_close_pending: bool,
    pub last_tray_click: Option<std::time::Instant>,
    pub connection_start: Option<Instant>,
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
                self.window_close_pending = false;
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
                    self.window_close_pending = false;
                }
                Task::none()
            }
            Message::ResetClosePending => {
                log::info!("Resetting window_close_pending (safety timeout)");
                self.window_close_pending = false;
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
                        // Debounce rapid clicks (Windows fires multiple events)
                        let now = std::time::Instant::now();
                        if let Some(last) = self.last_tray_click {
                            if now.duration_since(last) < std::time::Duration::from_millis(500) {
                                return Task::none();
                            }
                        }
                        self.last_tray_click = Some(now);
                        log::info!("Tray icon clicked, toggling visibility");
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
                    let now = std::time::Instant::now();
                    if let Some(last) = self.last_tray_click {
                        if now.duration_since(last) < std::time::Duration::from_millis(500) {
                            return Task::none();
                        }
                    }
                    self.last_tray_click = Some(now);
                    log::info!("Menu 'show' clicked, toggling visibility");
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
            Message::ClientDecorationsToggled(v) => {
                self.settings.use_client_decorations = v;
                self.save_settings();
                // Note: Window decoration changes require restart
                Task::none()
            }
            Message::ToggleVisibility { from_close_request } => {
                log::info!(
                    "ToggleVisibility called. visible={}, close_to_tray={}, from_close_request={}, close_pending={}",
                    self.is_visible,
                    self.settings.close_to_tray,
                    from_close_request,
                    self.window_close_pending
                );

                // Ignore toggles while a close is in-flight
                if self.window_close_pending {
                    return Task::none();
                }

                if self.is_visible {
                    if from_close_request && !self.settings.close_to_tray {
                        log::info!("Exiting application due to close request");
                        return iced::exit();
                    }
                    log::info!("Closing window to hide");
                    self.is_visible = false;
                    self.window_close_pending = true;
                    if let Some(id) = self.window_id {
                        return Task::batch(vec![
                            iced::window::close(id),
                            Task::perform(
                                async { tokio::time::sleep(std::time::Duration::from_millis(500)).await },
                                |_| Message::ResetClosePending,
                            ),
                        ]);
                    }
                } else {
                    log::info!("Opening window to show");
                    // Set visible immediately to prevent double-open
                    self.is_visible = true;
                    let use_csd = self.settings.use_client_decorations;
                    let (id, task) = iced::window::open(iced::window::Settings {
                        exit_on_close_request: false,
                        size: iced::Size::new(580.0, 650.0),
                        min_size: Some(iced::Size::new(560.0, 580.0)),
                        decorations: !use_csd,
                        transparent: use_csd,
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
            Message::Tick => {
                if self.status == ConnectionStatus::Connecting
                    || self.status == ConnectionStatus::Disconnecting
                {
                    self.rotation += 0.1;
                }
                Task::none()
            }
            // ConnectPressed handled above
            Message::ConnectPressed => {
                if self.status == ConnectionStatus::Disconnected
                    || self.status == ConnectionStatus::Error
                {
                    self.automation_warning = None; // Clear previous warnings
                    self.error_message = None;
                    self.status_message = "Initializing...".to_string();
                    self.connection_start = Some(Instant::now());
                    let (headless, no_auto_login) = login_mode_flags(self.settings.login_mode_val);

                    let config = SessionConfig {
                        url: self.settings.url.clone(),
                        domain: self.settings.domain.clone(),
                        user_agent: "Mozilla/5.0".to_string(),
                        headless,
                        no_auto_login,
                        email: if self.settings.email.is_empty() {
                            None
                        } else {
                            Some(self.settings.email.clone())
                        },
                        openconnect_path: if self.settings.openconnect_path.is_empty() {
                            "openconnect".to_string()
                        } else {
                            self.settings.openconnect_path.clone()
                        },
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
                            let (interaction_tx, mut interaction_rx) =
                                tokio::sync::mpsc::channel(10);

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

                                if session_c.is_finished() {
                                    break;
                                }

                                // Poll interactions
                                match interaction_rx.try_recv() {
                                    Ok(GuiInteraction::Request(req)) => {
                                        let _ = output
                                            .send(Message::RequestInput(Arc::new(
                                                InputRequestWrapper(Mutex::new(Some(req))),
                                            )))
                                            .await;
                                    }
                                    Ok(GuiInteraction::MfaPush(code)) => {
                                        let _ = output.send(Message::MfaPushReceived(code)).await;
                                    }
                                    Ok(GuiInteraction::MfaComplete) => {
                                        let _ = output.send(Message::MfaCompleteReceived).await;
                                    }
                                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                                        break
                                    }
                                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
                                }

                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            }
                            let _ = output
                                .send(Message::ConnectionFinished(session_c.last_error()))
                                .await;
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
                if let Some(parsed) = kuvpn::ParsedLog::parse(&raw_log) {
                    // Update UI status/error messages
                    match parsed.level {
                        log::Level::Error => self.error_message = Some(parsed.message.clone()),
                        log::Level::Warn | log::Level::Info => {
                            self.status_message = parsed.message.clone()
                        }
                        _ => {}
                    }

                    let user_filter = if let Ok(guard) = crate::logger::GUI_LOGGER.user_level.lock()
                    {
                        *guard
                    } else {
                        log::LevelFilter::Info
                    };

                    if parsed.level <= user_filter {
                        self.logs
                            .push(format!("[{}] {}", parsed.prefix(), parsed.message));
                        if self.logs.len() > 500 {
                            self.logs.remove(0);
                        }
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
                self.status = if err.is_some() {
                    ConnectionStatus::Error
                } else {
                    ConnectionStatus::Disconnected
                };
                self.mfa_info = None;
                self.connection_start = None;

                // Update tray icon based on final status
                if let Some(tray) = &self.tray_icon {
                    crate::tray::update_tray_icon(tray, self.status);
                }

                if let Some(e) = err {
                    self.error_message = Some(e.clone());
                    self.logs.push(format!("[!] Session Error: {}", e));

                    // Detect automation failure and set warning
                    if e.contains("AUTOMATION_FAILED") {
                        self.automation_warning = Some(
                            "Full Auto mode encountered repeated issues.\n\n\
                             Suggestions:\n\
                             • Switch to Manual mode to complete login yourself\n\
                             • Use Visual Auto mode and record a screen video for bug reporting\n\
                             • Try clearing session data (Wipe Session button)"
                                .to_string(),
                        );
                    }
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
                    // Update tray icon based on new status
                    if let Some(tray) = &self.tray_icon {
                        crate::tray::update_tray_icon(tray, status);
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
            Message::CopyLogs => {
                let logs_text = self.logs.join("\n");
                Task::perform(
                    async move {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(logs_text);
                            // Keep clipboard alive for 500ms to allow clipboard managers to access it
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                    },
                    |_| Message::Tick,
                )
            }
            Message::TabChanged(tab) => {
                self.current_tab = tab;
                Task::none()
            }
            Message::DragWindow => {
                if let Some(id) = self.window_id {
                    iced::window::drag(id)
                } else {
                    Task::none()
                }
            }
            Message::MinimizeWindow => {
                if let Some(id) = self.window_id {
                    iced::window::minimize(id, true)
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![];

        if self.status == ConnectionStatus::Connecting
            || self.status == ConnectionStatus::Disconnecting
        {
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
            current_tab: Tab::Connection,
            logs: vec!["Ready for secure campus access.".to_string()],
            status: ConnectionStatus::Disconnected,
            pending_request: None,
            current_input: String::new(),
            mfa_info: None,
            status_message: "Ready to connect".to_string(),
            error_message: None,
            rotation: 0.0,
            oc_test_result: None,
            automation_warning: None,
            session: None,
            tray_icon: None,
            show_item: None,
            connect_item: None,
            disconnect_item: None,
            window_id: None,
            is_visible: false,
            window_close_pending: false,
            last_tray_click: None,
            connection_start: None,
        }
    }
}
