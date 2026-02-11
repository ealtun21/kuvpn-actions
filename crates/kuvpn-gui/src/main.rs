use iced::widget::{button, column, container, row, scrollable, text, text_input, checkbox, pick_list};
use iced::{Alignment, Element, Length, Task, Font, stream};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use std::process::Stdio;
use std::io::{BufRead, BufReader};
use kuvpn::utils::CredentialsProvider;
use futures::SinkExt;

pub fn main() -> iced::Result {
    iced::run("KUVPN", KuVpnGui::update, KuVpnGui::view)
}

#[derive(Debug, Clone, PartialEq)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
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
}

#[derive(Debug)]
struct InputRequest {
    msg: String,
    is_password: bool,
    response_tx: oneshot::Sender<String>,
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
    DisconnectPressed,
    ClearSessionPressed,
    LogAppended(String),
    RequestInput(Arc<InputRequestWrapper>),
    InputChanged(String),
    SubmitInput,
    ConnectionFinished(Option<String>),
    StatusChanged(ConnectionStatus),
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
            Message::ConnectPressed => {
                if self.status == ConnectionStatus::Disconnected {
                    self.status = ConnectionStatus::Connecting;
                    self.logs.clear();
                    self.logs.push("[*] Starting connection process...".to_string());
                    
                    let url = self.url.clone();
                    let domain = self.domain.clone();
                    let headless = !self.show_browser;
                    let escalation_tool = self.escalation_tool.clone();
                    let (cancel_tx, mut cancel_rx) = oneshot::channel();
                    self.cancel_tx = Some(cancel_tx);

                    return Task::stream(stream::channel(100, move |mut output| async move {
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

                            let _ = log_tx_c.blocking_send("[*] Locating openconnect...".to_string());
                            let openconnect_path = match kuvpn::locate_openconnect("openconnect") {
                                Some(p) => p,
                                None => {
                                    let _ = log_tx_c.blocking_send("Error: Could not locate openconnect".to_string());
                                    return;
                                }
                            };

                            let _ = log_tx_c.blocking_send(format!("[*] Executing openconnect (escalation: {})...", escalation_tool));
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
                                    let _ = log_tx_c.blocking_send("[*] OpenConnect process finished".to_string());
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
                                    let _ = log_tx.send("[*] Disconnecting...".to_string()).await;
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
            Message::DisconnectPressed => {
                if let Some(tx) = self.cancel_tx.take() {
                    let _ = tx.send(());
                }
            }
            Message::ClearSessionPressed => {
                match kuvpn::get_user_data_dir() {
                    Ok(dir) => {
                        if dir.exists() {
                            if let Err(e) = std::fs::remove_dir_all(&dir) {
                                self.logs.push(format!("[!] Failed to clear session: {}", e));
                            } else {
                                self.logs.push("[✓] Session cleared successfully.".to_string());
                            }
                        } else {
                            self.logs.push("[*] No session found to clear.".to_string());
                        }
                    }
                    Err(e) => {
                        self.logs.push(format!("[!] Error getting session dir: {}", e));
                    }
                }
            }
            Message::StatusChanged(status) => {
                self.status = status;
            }
            Message::LogAppended(log) => {
                if log.contains("Push Approval: ") {
                    let cleaned = log.replace("[*] ", "").replace("Push Approval: ", "");
                    self.mfa_info = Some(cleaned);
                } else if log.contains("Push page finished") || log.contains("Number prompt gone") {
                    self.mfa_info = None;
                }
                self.logs.push(log);
                if self.logs.len() > 1000 {
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
                    self.logs.push(format!("[!] Error: {}", e));
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let title = text("KUVPN")
            .size(60)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .font(Font::with_name("sans-serif"));

        let status_color = match self.status {
            ConnectionStatus::Disconnected => iced::Color::from_rgb(0.5, 0.5, 0.5),
            ConnectionStatus::Connecting => iced::Color::from_rgb(1.0, 0.7, 0.0),
            ConnectionStatus::Connected => iced::Color::from_rgb(0.0, 0.8, 0.2),
        };

        let status_text = match self.status {
            ConnectionStatus::Disconnected => "Ready to Connect",
            ConnectionStatus::Connecting => "Connecting...",
            ConnectionStatus::Connected => "Connected",
        };

        let status_indicator = container(
            row![
                container(iced::widget::Space::with_width(12))
                    .height(12)
                    .width(12)
                    .style(move |_| container::Style::default()
                        .background(status_color)
                        .border(iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        })
                    ),
                text(status_text).size(16).color(status_color)
            ].spacing(10).align_y(Alignment::Center)
        ).width(Length::Fill).center_x(Length::Fill);

        let mfa_notice = if let Some(mfa) = &self.mfa_info {
            container(
                column![
                    text("Action Required").size(14).color(iced::Color::from_rgb(1.0, 1.0, 0.0)),
                    text(mfa).size(28).color(iced::Color::WHITE),
                ].spacing(5).align_x(Alignment::Center)
            )
            .padding(20)
            .width(Length::Fill)
            .style(|_| container::Style::default()
                .background(iced::Color::from_rgb(0.15, 0.15, 0.0))
                .border(iced::Border {
                    color: iced::Color::from_rgb(1.0, 1.0, 0.0),
                    width: 1.0,
                    radius: 8.0.into(),
                })
            )
        } else {
            container(iced::widget::Space::with_height(0))
        };

        let action_button: Element<Message> = match self.status {
            ConnectionStatus::Disconnected => button(
                text("Connect")
                    .size(24)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(15)
            .on_press(Message::ConnectPressed)
            .style(button::primary)
            .into(),
            ConnectionStatus::Connecting => button(
                text("Cancel")
                    .size(24)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(15)
            .on_press(Message::DisconnectPressed)
            .style(button::secondary)
            .into(),
            ConnectionStatus::Connected => container(iced::widget::Space::with_height(0)).into(),
        };

        let advanced_content = if self.show_advanced {
            container(
                column![
                    row![
                        text("VPN URL").width(Length::Fixed(100.0)),
                        text_input("URL", &self.url).on_input(Message::UrlChanged).padding(8),
                    ].spacing(10).align_y(Alignment::Center),
                    row![
                        text("DSID Domain").width(Length::Fixed(100.0)),
                        text_input("Domain", &self.domain).on_input(Message::DomainChanged).padding(8),
                    ].spacing(10).align_y(Alignment::Center),
                    row![
                        text("Escalation").width(Length::Fixed(100.0)),
                        pick_list(
                            vec!["pkexec".to_string(), "sudo".to_string(), "doas".to_string()],
                            Some(self.escalation_tool.clone()),
                            Message::EscalationToolChanged
                        ),
                    ].spacing(10).align_y(Alignment::Center),
                    row![
                        checkbox("Show automation browser", self.show_browser).on_toggle(Message::ShowBrowserToggled),
                    ].spacing(10),
                    button(text("Clear All Session Data"))
                        .on_press(Message::ClearSessionPressed)
                        .style(button::danger)
                        .padding(8),
                ].spacing(15)
            )
            .padding(20)
            .style(|_| container::Style::default()
                .background(iced::Color::from_rgb(0.1, 0.1, 0.1))
                .border(iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                })
            )
        } else {
            container(iced::widget::Space::with_height(0))
        };

        let console = if self.show_console {
            container(
                scrollable(
                    text(self.logs.join("\n"))
                        .font(Font::MONOSPACE)
                        .size(12)
                        .color(iced::Color::from_rgb(0.0, 0.8, 0.0)),
                )
                .height(Length::Fill),
            )
            .padding(10)
            .height(Length::Fixed(180.0))
            .width(Length::Fill)
            .style(|_| container::Style::default()
                .background(iced::Color::from_rgb(0.02, 0.02, 0.02))
                .border(iced::Border {
                    color: iced::Color::from_rgb(0.2, 0.2, 0.2),
                    width: 1.0,
                    radius: 4.0.into(),
                })
            )
        } else {
            container(iced::widget::Space::with_height(0))
        };

        let toggles = row![
            button(text(if self.show_advanced { "Hide Settings" } else { "Advanced Settings" }))
                .on_press(Message::ToggleAdvanced)
                .style(button::text),
            button(text(if self.show_console { "Hide Logs" } else { "Show Logs" }))
                .on_press(Message::ToggleConsole)
                .style(button::text),
        ].spacing(20);

        let mut content = column![
            title,
            status_indicator,
            mfa_notice,
            action_button,
            toggles,
        ].spacing(25).padding(40).align_x(Alignment::Center);

        if self.show_advanced {
            content = content.push(advanced_content);
        }

        if self.show_console {
            content = content.push(console);
        }

        let main_view = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill);

        if let Some(req) = &self.pending_request {
            let modal_content = container(
                column![
                    text("Authentication Required").size(20),
                    text(&req.msg).size(16),
                    if req.is_password {
                        text_input("Password", &self.current_input)
                            .on_input(Message::InputChanged)
                            .secure(true)
                            .on_submit(Message::SubmitInput)
                            .padding(12)
                    } else {
                        text_input("Enter value", &self.current_input)
                            .on_input(Message::InputChanged)
                            .on_submit(Message::SubmitInput)
                            .padding(12)
                    },
                    button(text("Submit").width(Length::Fill).align_x(iced::alignment::Horizontal::Center))
                        .padding(12)
                        .on_press(Message::SubmitInput)
                        .style(button::primary)
                ]
                .spacing(20)
                .padding(30)
            )
            .width(Length::Fixed(400.0))
            .style(|_| container::Style::default()
                .background(iced::Color::from_rgb(0.15, 0.15, 0.15))
                .border(iced::Border {
                    radius: 12.0.into(),
                    ..Default::default()
                })
            );

            let modal = container(modal_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style::default().background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.85)));
            
            return container(iced::widget::stack![main_view, modal])
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        main_view.into()
    }
}

impl Default for KuVpnGui {

    fn default() -> Self {

        Self {

            url: "https://vpn.ku.edu.tr".to_string(),

            domain: "vpn.ku.edu.tr".to_string(),

            show_browser: false,

            show_advanced: false,

            show_console: true,

            logs: vec!["Welcome to KUVPN".to_string()],

            status: ConnectionStatus::Disconnected,

            pending_request: None,

            current_input: String::new(),

            cancel_tx: None,

            mfa_info: None,

            escalation_tool: "pkexec".to_string(),

        }

    }

}
