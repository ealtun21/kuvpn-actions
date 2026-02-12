use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, stack, text, text_input, svg,
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

fn get_title(_: &KuVpnGui) -> String {
    "KUVPN".to_string()
}

fn get_theme(_: &KuVpnGui) -> iced::Theme {
    iced::Theme::Dark
}

fn init_tray() -> TrayIcon {
    let tray_menu = Menu::with_items(&[
        &MenuItem::with_id("show", "Show KUVPN", true, None),
        &MenuItem::with_id("connect", "Connect", true, None),
        &MenuItem::with_id("disconnect", "Disconnect", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("quit", "Quit", true, None),
    ]).expect("Failed to create tray menu");

    // Simple burgundy icon
    let mut rgba = vec![0u8; 32 * 32 * 4];
    for i in 0..32*32 {
        rgba[i*4] = 128;   // R
        rgba[i*4+1] = 0;   // G
        rgba[i*4+2] = 32;  // B
        rgba[i*4+3] = 255; // A
    }
    let icon = tray_icon::Icon::from_rgba(rgba, 32, 32).expect("Failed to create icon");

    TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("KUVPN")
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon")
}

pub fn main() -> iced::Result {
    #[cfg(target_os = "linux")]
    {
        // tray-icon on Linux requires GTK to be initialized
        let _ = gtk::init();
    }

    let tray = Arc::new(Mutex::new(Some(init_tray())));
    
    iced::application(move || {
        let mut gui = KuVpnGui::default();
        if let Ok(mut guard) = tray.lock() {
            gui.tray_icon = guard.take();
        }
        (gui, Task::none())
    }, KuVpnGui::update, KuVpnGui::view)
        .title(get_title)
        .font(NERD_FONT_BYTES)
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
    LogLevelChanged(String),
    OpenConnectPathChanged(String),
    EmailChanged(String),
    NoAutoLoginToggled(bool),
    ShowBrowserToggled(bool),
    ToggleAdvanced,
    ToggleConsole,
    ConnectPressed,
    DisconnectPressed,
    LogAppended(String),
    RequestInput(Arc<InputRequestWrapper>),
    InputChanged(String),
    SubmitInput,
    ClearSessionPressed,
    ConnectionFinished(Option<String>),
    StatusChanged(ConnectionStatus),
    Tick,
    TrayEvent(TrayIconEvent),
    MenuEvent(MenuEvent),
    CloseToTrayToggled(bool),
    ToggleVisibility,
    WindowOpened(iced::window::Id),
}

struct KuVpnGui {
    // Basic Settings
    url: String,
    domain: String,
    escalation_tool: String,
    
    // Advanced Settings (CLI parity)
    log_level: String,
    openconnect_path: String,
    no_auto_login: bool,
    email: String,
    show_browser: bool,

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
struct InputRequestWrapper(Mutex<Option<InputRequest>>);

static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

struct GuiProvider {
    request_tx: mpsc::Sender<InputRequest>,
}

impl CredentialsProvider for GuiProvider {
    fn request_text(&self, msg: &str) -> String {
        self.request(msg, false)
    }
    fn request_password(&self, msg: &str) -> String {
        self.request(msg, true)
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
        
        let _ = self.request_tx.blocking_send(request);
        futures::executor::block_on(rx).unwrap_or_default()
    }
}

struct GuiLogger {
    tx: Mutex<Option<mpsc::Sender<String>>>,
    user_level: Mutex<log::LevelFilter>,
}

impl log::Log for GuiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool { 
        // Always enable internal trace/debug for state tracking
        metadata.level() <= log::Level::Trace
    }
    fn log(&self, record: &log::Record) {
        // Internal state tracking always sees the logs
        // But we only send to the UI if it matches user preference
        let user_level = if let Ok(guard) = self.user_level.lock() {
            *guard
        } else {
            log::LevelFilter::Info
        };

        if record.level() <= user_level {
            if let Ok(guard) = self.tx.lock() {
                if let Some(tx) = &*guard {
                    let _ = tx.try_send(format!("[*] {}", record.args()));
                }
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
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(id) => {
                self.window_id = Some(id);
                Task::none()
            }
            Message::TrayEvent(event) => {
                match event {
                    TrayIconEvent::Click { .. } => {
                        self.is_visible = !self.is_visible;
                        let is_visible = self.is_visible;
                        if let Some(id) = self.window_id {
                            return iced::window::minimize(id, !is_visible);
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::MenuEvent(event) => {
                match event.id.as_ref() {
                    "quit" => return iced::exit(),
                    "show" => {
                        self.is_visible = true;
                        if let Some(id) = self.window_id {
                            return iced::window::minimize(id, false);
                        }
                        Task::none()
                    }
                    "connect" => self.update(Message::ConnectPressed),
                    "disconnect" => self.update(Message::DisconnectPressed),
                    _ => Task::none(),
                }
            }
            Message::CloseToTrayToggled(v) => {
                self.close_to_tray = v;
                Task::none()
            }
            Message::ToggleVisibility => {
                if !self.close_to_tray {
                    return iced::exit();
                }
                self.is_visible = !self.is_visible;
                let is_visible = self.is_visible;
                if let Some(id) = self.window_id {
                    return iced::window::minimize(id, !is_visible);
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
            Message::LogLevelChanged(lvl) => {
                self.log_level = lvl.clone();
                if let Ok(mut guard) = GUI_LOGGER.user_level.lock() {
                    *guard = match lvl.as_str() {
                        "off" => log::LevelFilter::Off,
                        "info" => log::LevelFilter::Info,
                        "warn" => log::LevelFilter::Warn,
                        "debug" => log::LevelFilter::Debug,
                        "error" => log::LevelFilter::Error,
                        "trace" => log::LevelFilter::Trace,
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
            Message::NoAutoLoginToggled(v) => {
                self.no_auto_login = v;
                Task::none()
            }
            Message::ShowBrowserToggled(show) => {
                self.show_browser = show;
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
                    let headless = !self.show_browser;
                    let escalation_tool = self.escalation_tool.clone();
                    let openconnect_path = if self.openconnect_path.is_empty() { "openconnect".to_string() } else { self.openconnect_path.clone() };
                    let no_auto_login = self.no_auto_login;
                    let email = if self.email.is_empty() { None } else { Some(self.email.clone()) };

                    let (cancel_tx, mut cancel_rx) = oneshot::channel();
                    self.cancel_tx = Some(cancel_tx);

                    return Task::stream(iced::stream::channel(100, move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                        let (log_tx, mut log_rx) = mpsc::channel(100);
                        let (req_tx, mut req_rx) = mpsc::channel(1);
                        let (child_tx, mut child_rx) = mpsc::channel::<Arc<Mutex<Option<std::process::Child>>>>(1);
                        
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
                            let provider = GuiProvider { request_tx: req_tx };
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
                                    let _ = log_tx_c.blocking_send(format!("Error: {}", e));
                                    return;
                                }
                            };

                            let _ = log_tx_c.blocking_send("[*] Initializing Koç University tunnel...".to_string());
                            let final_oc_path = match kuvpn::locate_openconnect(&openconnect_path) {
                                Some(p) => p,
                                None => {
                                    let _ = log_tx_c.blocking_send(format!("Error: Could not locate '{}'", openconnect_path));
                                    return;
                                }
                            };

                            match kuvpn::execute_openconnect(dsid, url_c, &Some(escalation_tool), &final_oc_path, Stdio::piped(), Stdio::piped()) {
                                Ok(mut child) => {
                                    let stdout = child.stdout.take().unwrap();
                                    let stderr = child.stderr.take().unwrap();
                                    
                                    let shared_child = Arc::new(Mutex::new(Some(child)));
                                    let _ = child_tx.blocking_send(shared_child.clone());

                                    let log_tx_stdout = log_tx_c.clone();
                                    std::thread::spawn(move || {
                                        let reader = BufReader::new(stdout);
                                        for line in reader.lines() {
                                            if let Ok(l) = line {
                                                let _ = log_tx_stdout.blocking_send(l);
                                            }
                                        }
                                    });

                                    let log_tx_stderr = log_tx_c.clone();
                                    std::thread::spawn(move || {
                                        let reader = BufReader::new(stderr);
                                        for line in reader.lines() {
                                            if let Ok(l) = line {
                                                let _ = log_tx_stderr.blocking_send(l);
                                            }
                                        }
                                    });

                                    if let Ok(mut guard) = shared_child.lock() {
                                        if let Some(child_ref) = guard.as_mut() {
                                            let _ = child_ref.wait();
                                        }
                                    };
                                }
                                Err(e) => {
                                    let _ = log_tx_c.blocking_send(format!("Error: {}", e));
                                }
                            }
                        });

                        let mut active_child: Option<Arc<Mutex<Option<std::process::Child>>>> = None;

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
                                res = req_rx.recv() => {
                                    if let Some(r) = res {
                                        let _ = output.send(Message::RequestInput(Arc::new(InputRequestWrapper(Mutex::new(Some(r)))))).await;
                                    } else {
                                        break;
                                    }
                                }
                                child_res = child_rx.recv() => {
                                    active_child = child_res;
                                }
                                _ = &mut cancel_rx => {
                                    if let Some(shared_child) = active_child.take() {
                                        if let Ok(mut guard) = shared_child.lock() {
                                            if let Some(mut child) = guard.take() {
                                                let _ = kuvpn::kill_child(&mut child);
                                            }
                                        }
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
            Message::LogAppended(log) => {
                let mfa_before = self.mfa_info.is_some();
                self.handle_log_event(&log);
                let mfa_after = self.mfa_info.is_some();
                
                self.logs.push(log);
                if self.logs.len() > 500 {
                    self.logs.remove(0);
                }

                if !mfa_before && mfa_after {
                    self.is_visible = true;
                    if let Some(id) = self.window_id {
                        return iced::window::minimize(id, false);
                    }
                }
                Task::none()
            }
            Message::RequestInput(wrapper) => {
                if let Ok(mut guard) = wrapper.0.lock() {
                    if let Some(req) = guard.take() {
                        self.pending_request = Some(req);
                        self.current_input = String::new();
                        self.is_visible = true;
                        if let Some(id) = self.window_id {
                            return iced::window::minimize(id, false);
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

    fn handle_log_event(&mut self, log: &str) {
        if log.contains("Push Approval: ") {
            let cleaned = log.replace("[*] ", "").replace("Push Approval: ", "");
            self.mfa_info = Some(cleaned);
        } else if log.contains("Push page finished") || log.contains("Number prompt gone") {
            self.mfa_info = None;
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![];

        if self.status == ConnectionStatus::Connecting {
            subs.push(iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick));
        }

        // Window events
        subs.push(iced::event::listen_with(|event, _status, window_id| {
            match event {
                iced::Event::Window(iced::window::Event::CloseRequested) => Some(Message::ToggleVisibility),
                iced::Event::Window(iced::window::Event::Opened { .. }) => Some(Message::WindowOpened(window_id)),
                _ => None,
            }
        }));

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
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            })
        }));

        Subscription::batch(subs)
    }

    fn view(&self) -> Element<'_, Message> {
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
                        text("Duo Approval Required").size(14).color(COLOR_WARNING).font(NERD_FONT),
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
                    pick_list(
                        vec!["off".to_string(), "info".to_string(), "warn".to_string(), "debug".to_string(), "error".to_string(), "trace".to_string()],
                        Some(self.log_level.clone()),
                        if is_locked { |_| Message::Tick } else { Message::LogLevelChanged }
                    ).width(Length::Fill),
                ].spacing(10).align_y(Alignment::Center),

                row![
                    text("Elevation:").width(Length::Fixed(120.0)),
                    pick_list(
                        vec!["pkexec".to_string(), "sudo".to_string(), "doas".to_string()],
                        Some(self.escalation_tool.clone()),
                        if is_locked { |_| Message::Tick } else { Message::EscalationToolChanged }
                    ).width(Length::Fill),
                ].spacing(10).align_y(Alignment::Center),
                
                row![
                    checkbox(self.show_browser)
                        .on_toggle(if is_locked { |_| Message::Tick } else { Message::ShowBrowserToggled }),
                    text("Disable automation headless mode"),
                ].spacing(10).align_y(Alignment::Center),

                row![
                    checkbox(self.no_auto_login)
                        .on_toggle(if is_locked { |_| Message::Tick } else { Message::NoAutoLoginToggled }),
                    text("Disable automatic login handlers"),
                ].spacing(10).align_y(Alignment::Center),

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
            log_level: "error".to_string(),
            openconnect_path: "openconnect".to_string(),
            no_auto_login: false,
            email: String::new(),
            show_browser: false,
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
            window_id: None,
            is_visible: true,
            close_to_tray: true,
        }
    }
}
