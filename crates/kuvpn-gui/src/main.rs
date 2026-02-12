use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, slider, stack, text, text_input, svg,
};
use iced::{Alignment, Border, Color, Element, Font, Length, Task, Subscription};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use std::process::Stdio;
use std::io::{BufRead, BufReader};
use kuvpn::utils::CredentialsProvider;
use futures::SinkExt;

// --- Constants & Styling ---
const NERD_FONT: Font = Font::with_name("JetBrainsMono Nerd Font Mono");
const NERD_FONT_BYTES: &[u8] = include_bytes!("../assets/JetBrainsMonoNerdFontMono-Regular.ttf");
const KU_LOGO_BYTES: &[u8] = include_bytes!("../assets/ku.svg");

// Colors (Refined Koç University Palette)
const COLOR_BG: Color = Color::from_rgb(0.07, 0.07, 0.07);
const COLOR_SURFACE: Color = Color::from_rgb(0.12, 0.12, 0.12);
const COLOR_ACCENT: Color = Color::from_rgb(0.50, 0.0, 0.125); // #800020 Burgundy
const COLOR_SUCCESS: Color = Color::from_rgb(0.42, 0.55, 0.35);
const COLOR_WARNING: Color = Color::from_rgb(0.80, 0.60, 0.30);
const COLOR_TEXT: Color = Color::from_rgb(0.85, 0.85, 0.85);
const COLOR_TEXT_DIM: Color = Color::from_rgb(0.50, 0.50, 0.50);

// Icons (Nerd Font)
const ICON_SETTINGS: &str = "\u{f013}";
const ICON_SHIELD: &str = "\u{f132}";
const ICON_SHIELD_CHECK: &str = "\u{f0568}";
const ICON_LOCK: &str = "\u{f023}";
const ICON_PHONE: &str = "\u{f095}";
const ICON_TERMINAL: &str = "\u{f120}";
const ICON_INFO: &str = "\u{f05a}";
const ICON_REFRESH: &str = "\u{f021}";
const ICON_TRASH: &str = "\u{f1f8}";

struct TrayComponents {
    tray: TrayIcon,
    show_item: MenuItem,
    connect_item: MenuItem,
    disconnect_item: MenuItem,
}

fn init_tray() -> TrayComponents {
    let show_item = MenuItem::with_id("show", "Hide KUVPN", true, None);
    let connect_item = MenuItem::with_id("connect", "Connect", true, None);
    let disconnect_item = MenuItem::with_id("disconnect", "Disconnect", true, None);
    let quit_item = MenuItem::with_id("quit", "Quit", true, None);

    let tray_menu = Menu::with_items(&[
        &show_item,
        &connect_item,
        &disconnect_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ]).expect("Failed to create tray menu");

    let icon = {
        let mut rgba = vec![0u8; 32 * 32 * 4];
        for i in 0..32*32 {
            rgba[i*4] = 128;   // R
            rgba[i*4+1] = 0;   // G
            rgba[i*4+2] = 32;  // B
            rgba[i*4+3] = 255; // A
        }
        tray_icon::Icon::from_rgba(rgba, 32, 32).expect("Failed to create icon")
    };

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("KUVPN")
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon");

    TrayComponents {
        tray,
        show_item,
        connect_item,
        disconnect_item,
    }
}

fn get_title(_: &KuVpnGui, _: iced::window::Id) -> String {
    "KUVPN".to_string()
}

fn get_theme(gui: &KuVpnGui, id: iced::window::Id) -> iced::Theme {
    gui.theme(id)
}

pub fn main() -> iced::Result {
    #[cfg(target_os = "linux")]
    {
        // tray-icon on Linux requires GTK to be initialized first
        let _ = gtk::init();
    }

    let components = Arc::new(Mutex::new(Some(init_tray())));
    
    iced::daemon(move || {
        let mut gui = KuVpnGui::default();
        if let Ok(mut guard) = components.lock() {
            if let Some(c) = guard.take() {
                gui.tray_icon = Some(c.tray);
                gui.show_item = Some(c.show_item);
                gui.connect_item = Some(c.connect_item);
                gui.disconnect_item = Some(c.disconnect_item);
            }
        }
        let (id, task) = iced::window::open(iced::window::Settings::default());
        gui.window_id = Some(id);
        (gui, Task::batch(vec![
            iced::font::load(NERD_FONT_BYTES).map(|_| Message::GtkTick),
            task.map(Message::WindowOpened)
        ]))
    }, KuVpnGui::update, KuVpnGui::view)
        .title(get_title)
        .default_font(NERD_FONT)
        .subscription(KuVpnGui::subscription)
        .theme(get_theme)
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone)]
enum Message {
    UrlChanged(String),
    DomainChanged(String),
    EscalationToolChanged(String),
    LogLevelSliderChanged(f32),
    OpenConnectPathChanged(String),
    EmailChanged(String),
    LoginModeChanged(f32),
    ToggleAdvanced,
    ToggleConsole,
    ConnectPressed,
    DisconnectPressed,
    LogAppended(String),
    RequestInput(Arc<InputRequestWrapper>),
    InputChanged(String),
    SubmitInput,
    MfaPushReceived(String),
    MfaCompleteReceived,
    ClearSessionPressed,
    ConnectionFinished(Option<String>),
    StatusChanged(ConnectionStatus),
    Tick,
    TrayEvent(TrayIconEvent),
    MenuEvent(MenuEvent),
    CloseToTrayToggled(bool),
    ToggleVisibility,
    WindowOpened(iced::window::Id),
    GtkTick,
}

struct KuVpnGui {
    // Basic Settings
    url: String,
    domain: String,
    escalation_tool: String,
    
    // Advanced Settings (CLI parity)
    log_level_val: f32,
    openconnect_path: String,
    login_mode_val: f32,
    email: String,

    // UI State
    show_advanced: bool,
    show_console: bool,
    logs: Vec<String>,
    status: ConnectionStatus,
    pending_request: Option<InputRequest>,
    current_input: String,
    cancel_tx: Option<oneshot::Sender<()>>,
    mfa_info: Option<String>,
    rotation: f32,
    
    // Tray & Window state
    tray_icon: Option<TrayIcon>,
    show_item: Option<MenuItem>,
    connect_item: Option<MenuItem>,
    disconnect_item: Option<MenuItem>,
    window_id: Option<iced::window::Id>,
    is_visible: bool,
    close_to_tray: bool,
}

#[derive(Debug)]
struct InputRequest {
    msg: String,
    is_password: bool,
    response_tx: oneshot::Sender<String>,
}

#[derive(Debug)]
enum GuiInteraction {
    Request(InputRequest),
    MfaPush(String),
    MfaComplete,
}

#[derive(Debug)]
struct InputRequestWrapper(Mutex<Option<InputRequest>>);

static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

struct GuiProvider {
    interaction_tx: mpsc::Sender<GuiInteraction>,
}

impl CredentialsProvider for GuiProvider {
    fn request_text(&self, msg: &str) -> String {
        self.request(msg, false)
    }
    fn request_password(&self, msg: &str) -> String {
        self.request(msg, true)
    }
    fn on_mfa_push(&self, code: &str) {
        let _ = self.interaction_tx.blocking_send(GuiInteraction::MfaPush(code.to_string()));
    }
    fn on_mfa_complete(&self) {
        let _ = self.interaction_tx.blocking_send(GuiInteraction::MfaComplete);
    }
}

impl GuiProvider {
    fn request(&self, msg: &str, is_password: bool) -> String {
        let (tx, rx) = oneshot::channel();
        let request = InputRequest {
            msg: msg.to_string(),
            is_password,
            response_tx: tx,
        };
        
        let _ = self.interaction_tx.blocking_send(GuiInteraction::Request(request));
        futures::executor::block_on(rx).unwrap_or_default()
    }
}

struct GuiLogger {
    tx: Mutex<Option<mpsc::Sender<String>>>,
    user_level: Mutex<log::LevelFilter>,
}

impl log::Log for GuiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool { 
        metadata.level() <= log::Level::Trace
    }
    fn log(&self, record: &log::Record) {
        if let Ok(guard) = self.tx.lock() {
            if let Some(tx) = &*guard {
                let _ = tx.try_send(format!("{:?}|{}", record.level(), record.args()));
            }
        }
    }
    fn flush(&self) {}
}

static GUI_LOGGER: GuiLogger = GuiLogger {
    tx: Mutex::new(None),
    user_level: Mutex::new(log::LevelFilter::Info),
};

impl KuVpnGui {
    fn theme(&self, _id: iced::window::Id) -> iced::Theme {
        iced::Theme::Dark
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(id) => {
                log::info!("Window opened with ID: {:?}", id);
                self.window_id = Some(id);
                self.is_visible = true;
                if let Some(item) = &self.show_item {
                    item.set_text("Hide KUVPN");
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
                    TrayIconEvent::Click { ..} => {
                        return self.update(Message::ToggleVisibility);
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::MenuEvent(event) => {
                match event.id.as_ref() {
                    "quit" => return iced::exit(),
                    "show" => {
                        return self.update(Message::ToggleVisibility);
                    }
                    "connect" => return self.update(Message::ConnectPressed),
                    "disconnect" => return self.update(Message::DisconnectPressed),
                    _ => Task::none(),
                }
            }
            Message::CloseToTrayToggled(v) => {
                self.close_to_tray = v;
                Task::none()
            }
            Message::ToggleVisibility => {
                log::info!("ToggleVisibility called. visible={}, close_to_tray={}", self.is_visible, self.close_to_tray);
                if self.is_visible {
                    if !self.close_to_tray {
                        log::info!("Exiting application");
                        return iced::exit();
                    }
                    log::info!("Closing window to hide");
                    self.is_visible = false;
                    if let Some(item) = &self.show_item {
                        item.set_text("Show KUVPN");
                    }
                    if let Some(id) = self.window_id.take() {
                        return iced::window::close(id);
                    }
                } else {
                    log::info!("Opening window to show");
                    let (id, task) = iced::window::open(iced::window::Settings::default());
                    self.window_id = Some(id);
                    return task.map(Message::WindowOpened);
                }
                Task::none()
            }
            Message::UrlChanged(url) => {
                self.url = url;
                Task::none()
            }
            Message::DomainChanged(domain) => {
                self.domain = domain;
                Task::none()
            }
            Message::EscalationToolChanged(tool) => {
                self.escalation_tool = tool;
                Task::none()
            }
            Message::LogLevelSliderChanged(val) => {
                self.log_level_val = val;
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
                Task::none()
            }
            Message::OpenConnectPathChanged(p) => {
                self.openconnect_path = p;
                Task::none()
            }
            Message::EmailChanged(e) => {
                self.email = e;
                Task::none()
            }
            Message::LoginModeChanged(val) => {
                self.login_mode_val = val;
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
            Message::ConnectPressed => {
                if self.status == ConnectionStatus::Disconnected {
                    self.status = ConnectionStatus::Connecting;
                    self.logs.clear();
                    self.logs.push("[*] Accessing campus gateway...".to_string());
                    
                    let url = self.url.clone();
                    let domain = self.domain.clone();
                    
                    let (headless, no_auto_login) = match self.login_mode_val.round() as i32 {
                        0 => (true, false),  // Full Automatic
                        1 => (false, false), // Visual Automatic
                        _ => (false, true),  // Manual
                    };

                    let escalation_tool = self.escalation_tool.clone();
                    let openconnect_path = if self.openconnect_path.is_empty() { "openconnect".to_string() } else { self.openconnect_path.clone() };
                    let email = if self.email.is_empty() { None } else { Some(self.email.clone()) };

                    let (cancel_tx, mut cancel_rx) = oneshot::channel();
                    self.cancel_tx = Some(cancel_tx);

                    return Task::stream(iced::stream::channel(100, move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
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

                        std::thread::spawn(move || {
                            let provider = GuiProvider { interaction_tx };
                            let dsid_res = kuvpn::run_login_and_get_dsid(
                                headless,
                                &url_c,
                                &domain_c,
                                "Mozilla/5.0",
                                no_auto_login,
                                email,
                                &provider,
                            );

                            let dsid = match dsid_res {
                                Ok(d) => d,
                                Err(e) => {
                                    let _ = log_tx_c.blocking_send(format!("Error|{}", e));
                                    return;
                                }
                            };

                            let _ = log_tx_c.blocking_send("Info|Initializing Koç University tunnel...".to_string());
                            let final_oc_path = match kuvpn::locate_openconnect(&openconnect_path) {
                                Some(p) => p,
                                None => {
                                    let _ = log_tx_c.blocking_send(format!("Error|Could not locate '{}'", openconnect_path));
                                    return;
                                }
                            };

                            match kuvpn::execute_openconnect(dsid, url_c, &Some(escalation_tool), &final_oc_path, Stdio::piped(), Stdio::piped()) {
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
                                                let _ = log_tx_stdout.blocking_send(format!("Info|{}", l));
                                            }
                                        }
                                    });

                                    let log_tx_stderr = log_tx_c.clone();
                                    std::thread::spawn(move || {
                                        let reader = BufReader::new(stderr);
                                        for line in reader.lines() {
                                            if let Ok(l) = line {
                                                let _ = log_tx_stderr.blocking_send(format!("Warn|{}", l));
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
                                    if let Some(pid) = active_pid.take() {
                                        let _ = kuvpn::kill_process(pid);
                                    }
                                    break;
                                }
                            }
                        }
                        let _ = output.send(Message::ConnectionFinished(None)).await;
                    }));
                }
                Task::none()
            }
            Message::DisconnectPressed => {
                if let Some(tx) = self.cancel_tx.take() {
                    let _ = tx.send(());
                }
                Task::none()
            }
            Message::LogAppended(raw_log) => {
                let parts: Vec<&str> = raw_log.splitn(2, '|').collect();
                if parts.len() < 2 { return Task::none(); }
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
                    return self.update(Message::ToggleVisibility);
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
                            return self.update(Message::ToggleVisibility);
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
                                self.logs.push(format!("[!] Failed to clear session: {}", e));
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
                self.status = status;
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![];

        if self.status == ConnectionStatus::Connecting {
            subs.push(iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick));
        }

        // GTK Event Loop pump (for Tray Icon on Linux)
        #[cfg(target_os = "linux")]
        subs.push(iced::time::every(std::time::Duration::from_millis(20)).map(|_| Message::GtkTick));

        // Window events
        subs.push(iced::window::close_requests().map(|_| Message::ToggleVisibility));

        // Tray & Menu events
        subs.push(Subscription::run(|| {
            iced::stream::channel(10, |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
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
            })
        }));

        Subscription::batch(subs)
    }

    fn view(&self, _id: iced::window::Id) -> Element<'_, Message> {
        let header = self.view_header();
        let status_view = self.view_status_circle();
        let mfa_banner = self.view_mfa_banner();
        let action_section = self.view_actions();
        
        let mut content = column![
            header,
            status_view,
            mfa_banner,
            action_section,
        ]
        .spacing(30)
        .padding(30)
        .align_x(Alignment::Center)
        .height(Length::Shrink);

        if self.show_advanced {
            content = content.push(self.view_advanced_settings());
        }

        if self.show_console {
            content = content.push(self.view_console());
        }

        let main_container = container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(COLOR_BG.into()),
                text_color: Some(COLOR_TEXT.into()),
                ..Default::default()
            });

        if let Some(req) = &self.pending_request {
            stack![
                main_container,
                self.view_modal(req)
            ].into()
        } else {
            main_container.into()
        }
    }

    fn view_header(&self) -> Element<'_, Message> {
        row![
            svg(svg::Handle::from_memory(KU_LOGO_BYTES)).width(32).height(32),
            text("KUVPN").size(24).font(NERD_FONT).width(Length::Fill),
            button(text(ICON_SETTINGS).font(NERD_FONT).size(24))
                .on_press(Message::ToggleAdvanced)
                .style(button::text),
            button(text(ICON_TERMINAL).font(NERD_FONT).size(24))
                .on_press(Message::ToggleConsole)
                .style(button::text),
        ]
        .spacing(15)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_status_circle(&self) -> Element<'_, Message> {
        let (color, icon, status_text) = match self.status {
            ConnectionStatus::Disconnected => (COLOR_TEXT_DIM, ICON_SHIELD, "Public Access"),
            ConnectionStatus::Connecting => (COLOR_WARNING, ICON_REFRESH, "Joining Campus..."),
            ConnectionStatus::Connected => (COLOR_SUCCESS, ICON_SHIELD_CHECK, "KU Network Active"),
        };

        let icon_display = text(icon).font(NERD_FONT).size(80).color(color);

        column![
            container(icon_display)
                .center_x(180)
                .center_y(180)
                .style(move |_| container::Style {
                    border: Border {
                        color: color,
                        width: 2.0,
                        radius: 90.0.into(),
                    },
                    ..Default::default()
                }),
            text(status_text)
                .size(18)
                .font(NERD_FONT)
                .color(color),
            text(if self.status == ConnectionStatus::Connected { "Internal Resources Available" } else { "Koç University Access Restricted" })
                .size(12)
                .color(COLOR_TEXT_DIM)
        ]
        .spacing(15)
        .align_x(Alignment::Center)
        .into()
    }

    fn view_mfa_banner(&self) -> Element<'_, Message> {
        if let Some(mfa) = &self.mfa_info {
            container(
                row![
                    text(ICON_PHONE).font(NERD_FONT).size(40).color(COLOR_WARNING),
                    column![
                        text("Approval Required").size(14).color(COLOR_WARNING).font(NERD_FONT),
                        text(mfa).size(22).color(Color::WHITE).font(NERD_FONT),
                    ].spacing(5)
                ]
                .spacing(20)
                .align_y(Alignment::Center)
            )
            .padding(20)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.80, 0.60, 0.30, 0.05).into()),
                border: Border {
                    color: COLOR_WARNING,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            })
            .into()
        } else {
            container(iced::widget::Space::new().height(0)).into()
        }
    }

    fn view_actions(&self) -> Element<'_, Message> {
        match self.status {
            ConnectionStatus::Disconnected => {
                button(
                    row![
                        svg(svg::Handle::from_memory(KU_LOGO_BYTES)).width(20).height(20),
                        text("JOIN NETWORK").font(NERD_FONT).size(16),
                    ].spacing(10).align_y(Alignment::Center)
                )
                .padding(15)
                .width(Length::Fixed(220.0))
                .on_press(Message::ConnectPressed)
                .style(button::primary)
                .into()
            }
            _ => {
                button(
                    row![
                        text("\u{f011}").font(NERD_FONT), // Power icon
                        text(if self.status == ConnectionStatus::Connecting { "CANCEL" } else { "DISCONNECT" })
                            .font(NERD_FONT).size(16),
                    ].spacing(10).align_y(Alignment::Center)
                )
                .padding(15)
                .width(Length::Fixed(220.0))
                .on_press(Message::DisconnectPressed)
                .style(button::secondary)
                .into()
            }
        }
    }

    fn view_advanced_settings(&self) -> Element<'_, Message> {
        let is_locked = self.status != ConnectionStatus::Disconnected;

        let locked_hint = if is_locked {
            container(
                row![
                    text(ICON_INFO).font(NERD_FONT).color(COLOR_ACCENT),
                    text("Settings locked during active session.").size(11).color(COLOR_TEXT_DIM),
                ].spacing(8).align_y(Alignment::Center)
            ).padding(5)
        } else {
            container(iced::widget::Space::new().height(0))
        };

        container(
            column![
                row![
                    text("CONFIGURATION").size(14).color(COLOR_ACCENT).font(NERD_FONT).width(Length::Fill),
                    locked_hint,
                ].align_y(Alignment::Center),
                
                // Parity with CLI options
                self.view_field("Gateway URL", &self.url, is_locked, Message::UrlChanged),
                self.view_field("DSID Domain", &self.domain, is_locked, Message::DomainChanged),
                self.view_field("Login Email", &self.email, is_locked, Message::EmailChanged),
                self.view_field("OC Path", &self.openconnect_path, is_locked, Message::OpenConnectPathChanged),
                
                row![
                    text("Log Level:").width(Length::Fixed(120.0)),
                    slider(0.0..=5.0, self.log_level_val, if is_locked { |_| Message::Tick } else { Message::LogLevelSliderChanged })
                        .step(1.0)
                        .width(Length::Fill),
                    text(match self.log_level_val.round() as i32 {
                        0 => "Off",
                        1 => "Error",
                        2 => "Warn",
                        3 => "Info",
                        4 => "Debug",
                        5 => "Trace",
                        _ => "Info",
                    }).width(Length::Fixed(60.0)),
                ].spacing(10).align_y(Alignment::Center),

                row![
                    text("Elevation:").width(Length::Fixed(120.0)),
                    pick_list(
                        vec!["pkexec".to_string(), "sudo".to_string(), "doas".to_string()],
                        Some(self.escalation_tool.clone()),
                        if is_locked { |_| Message::Tick } else { Message::EscalationToolChanged }
                    ).width(Length::Fill),
                ].spacing(10).align_y(Alignment::Center),
                
                column![
                    row![
                        text("Login Mode:").width(Length::Fixed(120.0)),
                        slider(0.0..=2.0, self.login_mode_val, if is_locked { |_| Message::Tick } else { Message::LoginModeChanged })
                            .step(1.0)
                            .width(Length::Fill),
                    ].spacing(10).align_y(Alignment::Center),
                    text(match self.login_mode_val.round() as i32 {
                        0 => "Full Automatic (Headless + Auto-Login)",
                        1 => "Visual Automatic (Browser + Auto-Login)",
                        _ => "Manual Mode (Browser + Manual Entry)",
                    })
                    .size(12)
                    .color(COLOR_TEXT_DIM)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
                ].spacing(5),

                row![
                    checkbox(self.close_to_tray)
                        .on_toggle(Message::CloseToTrayToggled),
                    text("Close window to system tray"),
                ].spacing(10).align_y(Alignment::Center),

                button(
                    row![
                        text(ICON_TRASH).font(NERD_FONT),
                        text("WIPE SAVED SESSION").font(NERD_FONT).size(12),
                    ].spacing(10).align_y(Alignment::Center)
                )
                .padding(10)
                .on_press(Message::ClearSessionPressed)
                .style(button::secondary),
            ]
            .spacing(12)
        )
        .padding(25)
        .style(|_| container::Style {
            background: Some(COLOR_SURFACE.into()),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }

    fn view_field(&self, label: &str, value: &str, locked: bool, on_change: fn(String) -> Message) -> Element<'_, Message> {
        row![
            text(format!("{}:", label)).width(Length::Fixed(120.0)),
            text_input(label, value)
                .on_input(if locked { |_| Message::Tick } else { on_change })
                .padding(8),
        ].spacing(10).align_y(Alignment::Center).into()
    }

    fn view_console(&self) -> Element<'_, Message> {
        container(
            column![
                row![
                    text(ICON_TERMINAL).font(NERD_FONT).color(COLOR_TEXT_DIM),
                    text("SESSION LOGS").size(12).color(COLOR_TEXT_DIM).font(NERD_FONT),
                ].spacing(10),
                scrollable(
                    text(self.logs.join("\n"))
                        .font(Font::MONOSPACE)
                        .size(11)
                        .color(COLOR_TEXT_DIM),
                )
                .height(Length::Fixed(140.0)),
            ].spacing(10)
        )
        .padding(15)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.04, 0.04, 0.04).into()),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }

    fn view_modal<'a>(&self, req: &'a InputRequest) -> Element<'a, Message> {
        let modal_content = container(
            column![
                row![
                    text(ICON_LOCK).font(NERD_FONT).size(30).color(COLOR_ACCENT),
                    text("Campus Gateway").size(24).font(NERD_FONT),
                ].spacing(15).align_y(Alignment::Center),
                
                text(&req.msg).size(16).color(COLOR_TEXT),
                
                if req.is_password {
                    text_input("Credentials", &self.current_input)
                        .on_input(Message::InputChanged)
                        .secure(true)
                        .on_submit(Message::SubmitInput)
                        .padding(15)
                } else {
                    text_input("Response Required", &self.current_input)
                        .on_input(Message::InputChanged)
                        .on_submit(Message::SubmitInput)
                        .padding(15)
                },
                
                button(
                    text("VERIFY")
                        .width(Length::Fill)
                        .align_x(Alignment::Center)
                        .font(NERD_FONT)
                )
                .padding(12)
                .on_press(Message::SubmitInput)
                .style(button::primary)
            ]
            .spacing(25)
            .padding(40)
        )
        .width(Length::Fixed(450.0))
        .style(|_| container::Style {
            background: Some(COLOR_SURFACE.into()),
            border: Border {
                radius: 12.0.into(),
                color: COLOR_ACCENT,
                width: 1.0,
            },
            ..Default::default()
        });

        container(modal_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.85).into()),
                ..Default::default()
            })
            .into()
    }
}

impl Default for KuVpnGui {
    fn default() -> Self {
        Self {
            url: "https://vpn.ku.edu.tr".to_string(),
            domain: "vpn.ku.edu.tr".to_string(),
            escalation_tool: "pkexec".to_string(),
            log_level_val: 1.0, // Default: Error
            openconnect_path: "openconnect".to_string(),
            login_mode_val: 0.0, // Default: Auto Headless
            email: String::new(),
            show_advanced: false,
            show_console: false,
            logs: vec!["Ready for secure campus access.".to_string()],
            status: ConnectionStatus::Disconnected,
            pending_request: None,
            current_input: String::new(),
            cancel_tx: None,
            mfa_info: None,
            rotation: 0.0,
            tray_icon: None,
            show_item: None,
            connect_item: None,
            disconnect_item: None,
            window_id: None,
            is_visible: false,
            close_to_tray: true,
        }
    }
}
