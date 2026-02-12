use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, stack, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Font, Length, Task, Subscription};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use std::process::Stdio;
use std::io::{BufRead, BufReader};
use kuvpn::utils::CredentialsProvider;
use futures::SinkExt;

// --- Constants & Styling ---
const NERD_FONT: Font = Font::with_name("JetBrainsMono Nerd Font Mono");
const NERD_FONT_BYTES: &[u8] = include_bytes!("../assets/JetBrainsMonoNerdFontMono-Regular.ttf");

// Colors (Nord/TokyoNight inspired)
const COLOR_BG: Color = Color::from_rgb(0.09, 0.10, 0.15);
const COLOR_SURFACE: Color = Color::from_rgb(0.14, 0.16, 0.23);
const COLOR_ACCENT: Color = Color::from_rgb(0.48, 0.64, 1.0);
const COLOR_SUCCESS: Color = Color::from_rgb(0.62, 0.81, 0.42);
const COLOR_WARNING: Color = Color::from_rgb(0.88, 0.69, 0.41);
const COLOR_TEXT: Color = Color::from_rgb(0.75, 0.79, 0.96);
const COLOR_TEXT_DIM: Color = Color::from_rgb(0.44, 0.47, 0.64);

// Icons (Nerd Font)
const ICON_VPN: &str = "\u{f0573}";
const ICON_SETTINGS: &str = "\u{f013}";
const ICON_SHIELD: &str = "\u{f132}";
const ICON_SHIELD_CHECK: &str = "\u{f0568}";
const ICON_LOCK: &str = "\u{f023}";
const ICON_PHONE: &str = "\u{f095}";
const ICON_TERMINAL: &str = "\u{f120}";
const ICON_REFRESH: &str = "\u{f021}";

pub fn main() -> iced::Result {
    iced::application("KUVPN", KuVpnGui::update, KuVpnGui::view)
        .font(NERD_FONT_BYTES)
        .subscription(KuVpnGui::subscription)
        .theme(|_| iced::Theme::Dark)
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
    ShowBrowserToggled(bool),
    ToggleAdvanced,
    ToggleConsole,
    ConnectPressed,
    LogAppended(String),
    RequestInput(Arc<InputRequestWrapper>),
    InputChanged(String),
    SubmitInput,
    ConnectionFinished(Option<String>),
    StatusChanged(ConnectionStatus),
    Tick,
}

struct KuVpnGui {
    url: String,
    domain: String,
    show_browser: bool,
    show_advanced: bool,
    show_console: bool,
    logs: Vec<String>,
    status: ConnectionStatus,
    pending_request: Option<InputRequest>,
    current_input: String,
    cancel_tx: Option<oneshot::Sender<()>>,
    mfa_info: Option<String>,
    escalation_tool: String,
    rotation: f32,
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
    tx: Mutex<Option<mpsc::Sender<String>>>
}

impl log::Log for GuiLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool { true }
    fn log(&self, record: &log::Record) {
        if let Ok(guard) = self.tx.lock() {
            if let Some(tx) = &*guard {
                let _ = tx.try_send(format!("[*] {}", record.args()));
            }
        }
    }
    fn flush(&self) {}
}

static GUI_LOGGER: GuiLogger = GuiLogger { tx: Mutex::new(None) };

impl KuVpnGui {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::UrlChanged(url) => self.url = url,
            Message::DomainChanged(domain) => self.domain = domain,
            Message::EscalationToolChanged(tool) => self.escalation_tool = tool,
            Message::ShowBrowserToggled(show) => self.show_browser = show,
            Message::ToggleAdvanced => self.show_advanced = !self.show_advanced,
            Message::ToggleConsole => self.show_console = !self.show_console,
            Message::Tick => {
                if self.status == ConnectionStatus::Connecting {
                    self.rotation += 0.1;
                }
            }
            Message::ConnectPressed => {
                if self.status == ConnectionStatus::Disconnected {
                    self.status = ConnectionStatus::Connecting;
                    self.logs.clear();
                    self.logs.push("[*] Initiating secure tunnel...".to_string());
                    
                    let url = self.url.clone();
                    let domain = self.domain.clone();
                    let headless = !self.show_browser;
                    let escalation_tool = self.escalation_tool.clone();
                    let (cancel_tx, mut cancel_rx) = oneshot::channel();
                    self.cancel_tx = Some(cancel_tx);

                    return Task::stream(iced::stream::channel(100, move |mut output| async move {
                        let (log_tx, mut log_rx) = mpsc::channel(100);
                        let (req_tx, mut req_rx) = mpsc::channel(1);
                        let (child_tx, mut child_rx) = mpsc::channel::<Arc<Mutex<Option<std::process::Child>>>>(1);
                        
                        if let Ok(mut guard) = GUI_LOGGER.tx.lock() {
                            *guard = Some(log_tx.clone());
                        }

                        LOGGER_INIT.call_once(|| {
                            let _ = log::set_logger(&GUI_LOGGER);
                            log::set_max_level(log::LevelFilter::Info);
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
                                false,
                                None,
                                &provider,
                            );

                            let dsid = match dsid_res {
                                Ok(d) => d,
                                Err(e) => {
                                    let _ = log_tx_c.blocking_send(format!("Error: {}", e));
                                    return;
                                }
                            };

                            let _ = log_tx_c.blocking_send("[*] Locating OpenConnect...".to_string());
                            let openconnect_path = match kuvpn::locate_openconnect("openconnect") {
                                Some(p) => p,
                                None => {
                                    let _ = log_tx_c.blocking_send("Error: Could not locate openconnect".to_string());
                                    return;
                                }
                            };

                            let _ = log_tx_c.blocking_send(format!("[*] Starting tunnel via {}...", escalation_tool));
                            match kuvpn::execute_openconnect(dsid, url_c, &Some(escalation_tool), &openconnect_path, Stdio::piped(), Stdio::piped()) {
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
                                    }
                                    let _ = log_tx_c.blocking_send("[*] Tunnel disconnected".to_string());
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
                                                let _ = child.kill();
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
            }
            Message::StatusChanged(status) => {
                self.status = status;
            }
            Message::LogAppended(log) => {
                self.handle_log_event(&log);
                self.logs.push(log);
                if self.logs.len() > 500 {
                    self.logs.remove(0);
                }
            }
            Message::RequestInput(wrapper) => {
                if let Ok(mut guard) = wrapper.0.lock() {
                    if let Some(req) = guard.take() {
                        self.pending_request = Some(req);
                        self.current_input = String::new();
                    }
                }
            }
            Message::InputChanged(val) => {
                self.current_input = val;
            }
            Message::SubmitInput => {
                if let Some(req) = self.pending_request.take() {
                    let _ = req.response_tx.send(self.current_input.clone());
                    self.current_input = String::new();
                }
            }
            Message::ConnectionFinished(err) => {
                self.status = ConnectionStatus::Disconnected;
                self.mfa_info = None;
                if let Some(e) = err {
                    self.logs.push(format!("[!] Critical: {}", e));
                }
            }
        }
        Task::none()
    }

    fn handle_log_event(&mut self, log: &str) {
        // MFA Detection logic - easily extendible
        if log.contains("Push Approval: ") {
            let cleaned = log.replace("[*] ", "").replace("Push Approval: ", "");
            self.mfa_info = Some(cleaned);
        } else if log.contains("Push page finished") || log.contains("Number prompt gone") {
            self.mfa_info = None;
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.status == ConnectionStatus::Connecting {
            iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
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
            text(ICON_VPN).font(NERD_FONT).size(32).color(COLOR_ACCENT),
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
            ConnectionStatus::Disconnected => (COLOR_TEXT_DIM, ICON_SHIELD, "Protected"),
            ConnectionStatus::Connecting => (COLOR_WARNING, ICON_REFRESH, "Securing..."),
            ConnectionStatus::Connected => (COLOR_SUCCESS, ICON_SHIELD_CHECK, "Secure Connection"),
        };

        let icon_display = text(icon).font(NERD_FONT).size(80).color(color);

        column![
            container(icon_display)
                .center_x(180)
                .center_y(180)
                .style(move |_| container::Style {
                    border: Border {
                        color: color,
                        width: 3.0,
                        radius: 90.0.into(),
                    },
                    ..Default::default()
                }),
            text(status_text)
                .size(20)
                .font(NERD_FONT)
                .color(color),
            if self.status == ConnectionStatus::Connected {
                text(&self.url).size(14).color(COLOR_TEXT_DIM)
            } else {
                text("Not Connected").size(14).color(COLOR_TEXT_DIM)
            }
        ]
        .spacing(20)
        .align_x(Alignment::Center)
        .into()
    }

    fn view_mfa_banner(&self) -> Element<'_, Message> {
        if let Some(mfa) = &self.mfa_info {
            container(
                row![
                    text(ICON_PHONE).font(NERD_FONT).size(40).color(COLOR_WARNING),
                    column![
                        text("MFA Action Required").size(16).color(COLOR_WARNING).font(NERD_FONT),
                        text(mfa).size(24).color(Color::WHITE).font(NERD_FONT),
                    ].spacing(5)
                ]
                .spacing(20)
                .align_y(Alignment::Center)
            )
            .padding(20)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.88, 0.69, 0.41, 0.1).into()),
                border: Border {
                    color: COLOR_WARNING,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            })
            .into()
        } else {
            container(iced::widget::Space::with_height(0)).into()
        }
    }

    fn view_actions(&self) -> Element<'_, Message> {
        match self.status {
            ConnectionStatus::Disconnected => {
                button(
                    row![
                        text(ICON_VPN).font(NERD_FONT),
                        text("CONNECT").font(NERD_FONT).size(18),
                    ].spacing(10).align_y(Alignment::Center)
                )
                .padding(15)
                .width(Length::Fixed(200.0))
                .on_press(Message::ConnectPressed)
                .style(button::primary)
                .into()
            }
            _ => {
                container(
                   text("VPN is running...")
                    .size(16)
                    .color(COLOR_TEXT_DIM)
                    .font(NERD_FONT)
                )
                .padding(15)
                .into()
            }
        }
    }

    fn view_advanced_settings(&self) -> Element<'_, Message> {
        container(
            column![
                text("ADVANCED SETTINGS").size(14).color(COLOR_ACCENT).font(NERD_FONT),
                row![
                    text("Gateway:").width(Length::Fixed(100.0)),
                    text_input("URL", &self.url).on_input(Message::UrlChanged).padding(10),
                ].spacing(10).align_y(Alignment::Center),
                row![
                    text("Domain:").width(Length::Fixed(100.0)),
                    text_input("Domain", &self.domain).on_input(Message::DomainChanged).padding(10),
                ].spacing(10).align_y(Alignment::Center),
                row![
                    text("Privilege:").width(Length::Fixed(100.0)),
                    pick_list(
                        vec!["pkexec".to_string(), "sudo".to_string(), "doas".to_string()],
                        Some(self.escalation_tool.clone()),
                        Message::EscalationToolChanged
                    ),
                ].spacing(10).align_y(Alignment::Center),
                checkbox("Show browser window", self.show_browser).on_toggle(Message::ShowBrowserToggled),
            ]
            .spacing(15)
        )
        .padding(25)
        .style(|_| container::Style {
            background: Some(COLOR_SURFACE.into()),
            border: Border {
                radius: 12.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }

    fn view_console(&self) -> Element<'_, Message> {
        container(
            column![
                row![
                    text(ICON_TERMINAL).font(NERD_FONT).color(COLOR_TEXT_DIM),
                    text("SYSTEM LOGS").size(12).color(COLOR_TEXT_DIM).font(NERD_FONT),
                ].spacing(10),
                scrollable(
                    text(self.logs.join("\n"))
                        .font(Font::MONOSPACE)
                        .size(11)
                        .color(COLOR_TEXT_DIM),
                )
                .height(Length::Fixed(150.0)),
            ].spacing(10)
        )
        .padding(15)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.05, 0.05, 0.08).into()),
            border: Border {
                radius: 8.0.into(),
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
                    text("Authentication").size(24).font(NERD_FONT),
                ].spacing(15).align_y(Alignment::Center),
                
                text(&req.msg).size(16).color(COLOR_TEXT),
                
                if req.is_password {
                    text_input("Password", &self.current_input)
                        .on_input(Message::InputChanged)
                        .secure(true)
                        .on_submit(Message::SubmitInput)
                        .padding(15)
                } else {
                    text_input("Enter response", &self.current_input)
                        .on_input(Message::InputChanged)
                        .on_submit(Message::SubmitInput)
                        .padding(15)
                },
                
                button(
                    text("SUBMIT")
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center)
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
                radius: 16.0.into(),
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
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.8).into()),
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
            show_browser: false,
            show_advanced: false,
            show_console: false,
            logs: vec!["Ready to secure your connection.".to_string()],
            status: ConnectionStatus::Disconnected,
            pending_request: None,
            current_input: String::new(),
            cancel_tx: None,
            mfa_info: None,
            escalation_tool: "pkexec".to_string(),
            rotation: 0.0,
        }
    }
}
