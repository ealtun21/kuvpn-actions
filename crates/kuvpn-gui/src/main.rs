use iced::widget::{button, column, container, row, scrollable, text, text_input, checkbox};
use iced::{Alignment, Element, Length, Task, Font, stream, Theme};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};
use std::process::Stdio;
use std::io::{BufRead, BufReader};
use kuvpn::utils::CredentialsProvider;
use futures::SinkExt;

pub fn main() -> iced::Result {
    iced::run("KUVPN GUI", KuVpnGui::update, KuVpnGui::view)
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
    email: String,
    headless: bool,
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
    EmailChanged(String),
    EscalationToolChanged(String),
    HeadlessToggled(bool),
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
    tx: Mutex<Option<mpsc::Sender<String>>>,
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
            Message::UrlChanged(url) => {
                self.url = url;
            }
            Message::DomainChanged(domain) => {
                self.domain = domain;
            }
            Message::EmailChanged(email) => {
                self.email = email;
            }
            Message::EscalationToolChanged(tool) => {
                self.escalation_tool = tool;
            }
            Message::HeadlessToggled(headless) => {
                self.headless = headless;
            }
            Message::ConnectPressed => {
                if self.status == ConnectionStatus::Disconnected {
                    self.status = ConnectionStatus::Connecting;
                    self.logs.clear();
                    self.logs.push("[*] Starting connection process...".to_string());
                    
                    let url = self.url.clone();
                    let domain = self.domain.clone();
                    let email = if self.email.is_empty() { None } else { Some(self.email.clone()) };
                    let headless = self.headless;
                    let escalation_tool = self.escalation_tool.clone();
                    let (cancel_tx, mut cancel_rx) = oneshot::channel();
                    self.cancel_tx = Some(cancel_tx);

                    return Task::stream(stream::channel(100, move |mut output| async move {
                        let (log_tx, mut log_rx) = mpsc::channel(100);
                        let (req_tx, mut req_rx) = mpsc::channel(1);
                        
                        // Update logger sender
                        if let Ok(mut guard) = GUI_LOGGER.tx.lock() {
                            *guard = Some(log_tx.clone());
                        }

                        // Set up logger once
                        LOGGER_INIT.call_once(|| {
                            let _ = log::set_logger(&GUI_LOGGER);
                            log::set_max_level(log::LevelFilter::Info);
                        });

                        let url_c = url.clone();
                        let domain_c = domain.clone();
                        let email_c = email.clone();
                        let headless_c = headless;
                        let log_tx_c = log_tx.clone();

                        let (child_tx, mut child_rx) = mpsc::channel::<Arc<Mutex<Option<std::process::Child>>>>(1);

                        std::thread::spawn(move || {
                            let provider = GuiProvider { request_tx: req_tx };
                            let dsid_res = kuvpn::run_login_and_get_dsid(
                                !headless_c,
                                &url_c,
                                &domain_c,
                                "Mozilla/5.0",
                                false,
                                email_c,
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

                            let _ = log_tx_c.blocking_send("[*] Executing openconnect...".to_string());
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
                    self.mfa_info = Some(log.clone());
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
            .size(40)
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center);

        let inputs = column![
            row![
                text("VPN URL:").width(Length::Fixed(120.0)),
                text_input("https://vpn.ku.edu.tr", &self.url)
                    .on_input(Message::UrlChanged)
                    .padding(10),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                text("DSID Domain:").width(Length::Fixed(120.0)),
                text_input("vpn.ku.edu.tr", &self.domain)
                    .on_input(Message::DomainChanged)
                    .padding(10),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                text("Email:").width(Length::Fixed(120.0)),
                text_input("user@ku.edu.tr (optional)", &self.email)
                    .on_input(Message::EmailChanged)
                    .padding(10),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                text("Headless Mode:").width(Length::Fixed(120.0)),
                checkbox("Use headless browser for login", self.headless).on_toggle(Message::HeadlessToggled),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                text("Escalation Tool:").width(Length::Fixed(120.0)),
                iced::widget::pick_list(
                    vec!["pkexec".to_string(), "sudo".to_string(), "doas".to_string()],
                    Some(self.escalation_tool.clone()),
                    Message::EscalationToolChanged
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(15);

        let mfa_notice = if let Some(mfa) = &self.mfa_info {
            container(
                text(mfa)
                    .size(20)
                    .color(iced::Color::from_rgb(1.0, 1.0, 0.0))
                    .align_x(iced::alignment::Horizontal::Center)
                    .width(Length::Fill),
            )
            .padding(10)
            .style(|_theme: &Theme| container::Style::default()
                .background(iced::Color::from_rgb(0.2, 0.2, 0.0))
                .border(iced::Border {
                    color: iced::Color::from_rgb(1.0, 1.0, 0.0),
                    width: 2.0,
                    radius: 8.0.into(),
                })
            )
        } else {
            container(iced::widget::Space::with_height(0))
        };

        let connect_button = match self.status {
            ConnectionStatus::Disconnected => button(
                text("Connect to VPN")
                    .size(18)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(15)
            .on_press(Message::ConnectPressed),
            ConnectionStatus::Connecting => button(
                text("Establishing Connection...")
                    .size(18)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(15),
            ConnectionStatus::Connected => button(
                text("Connected - Disconnect")
                    .size(18)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(15)
            .on_press(Message::DisconnectPressed)
            .style(button::danger),
        };

        let clear_button = button(
            text("Clear Session")
                .align_x(iced::alignment::Horizontal::Center),
        )
        .padding(10)
        .on_press(Message::ClearSessionPressed)
        .style(button::secondary);

        let actions = column![connect_button, clear_button].spacing(10).align_x(Alignment::Center);

        let console_content = self.logs.join("\n");
        let console = container(
            scrollable(
                text(console_content)
                    .font(Font::MONOSPACE)
                    .size(13)
                    .color(iced::Color::from_rgb(0.0, 1.0, 0.0)), // Green terminal text
            )
            .height(Length::Fill),
        )
        .padding(10)
        .height(Length::Fill)
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style::default()
            .background(iced::Color::from_rgb(0.05, 0.05, 0.05)) // Dark background
            .border(iced::Border {
                color: iced::Color::from_rgb(0.2, 0.2, 0.2),
                width: 1.0,
                radius: 4.0.into(),
            })
        );

        let main_content = column![title, inputs, mfa_notice, actions, console]
            .spacing(20)
            .padding(20);

        if let Some(req) = &self.pending_request {
            let modal_content = container(
                column![
                    text(&req.msg).size(18),
                    if req.is_password {
                        text_input("Enter password...", &self.current_input)
                            .on_input(Message::InputChanged)
                            .secure(true)
                            .on_submit(Message::SubmitInput)
                            .padding(10)
                    } else {
                        text_input("Enter value...", &self.current_input)
                            .on_input(Message::InputChanged)
                            .on_submit(Message::SubmitInput)
                            .padding(10)
                    },
                    button("Submit")
                        .width(Length::Fill)
                        .padding(10)
                        .on_press(Message::SubmitInput)
                ]
                .spacing(20)
                .padding(20)
            )
            .width(Length::Fixed(400.0))
            .style(container::dark);

            let modal = container(modal_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_theme: &Theme| container::Style::default().background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.8)));
            
            return container(iced::widget::stack![main_content, modal])
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        container(main_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Default for KuVpnGui {
    fn default() -> Self {
        Self {
            url: "https://vpn.ku.edu.tr".to_string(),
            domain: "vpn.ku.edu.tr".to_string(),
            email: "".to_string(),
            headless: true,
            logs: vec!["Welcome to KUVPN GUI".to_string()],
            status: ConnectionStatus::Disconnected,
            pending_request: None,
            current_input: String::new(),
            cancel_tx: None,
            mfa_info: None,
            escalation_tool: "pkexec".to_string(),
        }
    }
}
