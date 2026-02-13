use iced::{Color, Font};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tray_icon::{menu::MenuEvent, TrayIconEvent};

// --- Constants & Styling ---
pub const NERD_FONT: Font = Font {
    family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
    weight: iced::font::Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};
pub const NERD_FONT_BYTES: &[u8] =
    include_bytes!("../assets/JetBrainsMonoNerdFontMono-Regular.ttf");
pub const KU_LOGO_BYTES: &[u8] = include_bytes!("../assets/ku.svg");

// Colors (Refined Koç University Palette)
pub const COLOR_BG: Color = Color::from_rgb(0.07, 0.07, 0.07);
pub const COLOR_SURFACE: Color = Color::from_rgb(0.12, 0.12, 0.12);
pub const COLOR_ACCENT: Color = Color::from_rgb(0.50, 0.0, 0.125); // #800020 Burgundy
pub const COLOR_SUCCESS: Color = Color::from_rgb(0.42, 0.55, 0.35);
pub const COLOR_WARNING: Color = Color::from_rgb(0.80, 0.60, 0.30);
pub const COLOR_TEXT: Color = Color::from_rgb(0.85, 0.85, 0.85);
pub const COLOR_TEXT_DIM: Color = Color::from_rgb(0.50, 0.50, 0.50);

// Icons (Nerd Font)
pub const ICON_SETTINGS: &str = "\u{f013}";
pub const ICON_SHIELD: &str = "\u{f132}";
pub const ICON_SHIELD_CHECK: &str = "\u{f0568}";
pub const ICON_LOCK: &str = "\u{f023}";
pub const ICON_PHONE: &str = "\u{f095}";
pub const ICON_TERMINAL: &str = "\u{f120}";
pub const ICON_INFO: &str = "\u{f05a}";
pub const ICON_REFRESH: &str = "\u{f021}";
pub const ICON_TRASH: &str = "\u{f1f8}";

pub use kuvpn::ConnectionStatus;

#[derive(Debug, Clone)]
pub enum Message {
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
    ToggleVisibility { from_close_request: bool },
    WindowOpened(iced::window::Id),
    WindowClosed(iced::window::Id),
    GtkTick,
    ResetSettings,
    TestOpenConnect,
    OpenConnectTestResult(bool),
}

#[derive(Debug)]
pub struct InputRequest {
    pub msg: String,
    pub is_password: bool,
    pub response_tx: oneshot::Sender<String>,
}

#[derive(Debug)]
pub struct InputRequestWrapper(pub Mutex<Option<InputRequest>>);
