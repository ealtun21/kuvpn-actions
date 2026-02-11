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

struct KuVpnGui {
    url: String,
    domain: String,
    email: String,
    headless: bool,
    logs: Vec<String>,
    is_connecting: bool,
    pending_request: Option<InputRequest>,
    current_input: String,
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
    HeadlessToggled(bool),
    ConnectPressed,
    LogAppended(String),
    RequestInput(Arc<InputRequestWrapper>),
    InputChanged(String),
    SubmitInput,
    ConnectionFinished(Option<String>),
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
            Message::HeadlessToggled(headless) => {
                self.headless = headless;
            }
            Message::ConnectPressed => {
                if !self.is_connecting {
                    self.is_connecting = true;
                    self.logs.clear();
                    self.logs.push("[*] Starting connection process...".to_string());
                    
                    let url = self.url.clone();
                    let domain = self.domain.clone();
                    let email = if self.email.is_empty() { None } else { Some(self.email.clone()) };
                    let headless = self.headless;

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

                        std::thread::spawn(move || {
                            let provider = GuiProvider { request_tx: req_tx };
                            let dsid_res = kuvpn::run_login_and_get_dsid(
                                headless_c,
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
                            match kuvpn::execute_openconnect(dsid, url_c, &None, &openconnect_path, Stdio::piped(), Stdio::piped()) {
                                Ok(mut child) => {
                                    let stdout = child.stdout.take().unwrap();
                                    let stderr = child.stderr.take().unwrap();
                                    
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

                                    let _ = child.wait();
                                    let _ = log_tx_c.blocking_send("[*] OpenConnect process finished".to_string());
                                }
                                Err(e) => {
                                    let _ = log_tx_c.blocking_send(format!("Error: {}", e));
                                }
                            }
                        });

                        while let Some(msg) = tokio::select! {
                            res = log_rx.recv() => res.map(Message::LogAppended),
                            res = req_rx.recv() => res.map(|r| Message::RequestInput(Arc::new(InputRequestWrapper(Mutex::new(Some(r)))))),
                        } {
                            let _ = output.send(msg).await;
                        }
                        
                        let _ = output.send(Message::ConnectionFinished(None)).await;
                    }));
                }
            }
            Message::LogAppended(log) => {
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
                self.is_connecting = false;
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
        ]
        .spacing(15);

        let connect_button = button(
            text(if self.is_connecting {
                "Establishing Connection..."
            } else {
                "Connect to VPN"
            })
            .size(18)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .padding(15)
        .on_press(Message::ConnectPressed);

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

        let main_content = column![title, inputs, connect_button, console]
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
            is_connecting: false,
            pending_request: None,
            current_input: String::new(),
        }
    }
}
