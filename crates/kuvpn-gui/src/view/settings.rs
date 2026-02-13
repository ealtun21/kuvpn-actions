use crate::app::KuVpnGui;
use crate::types::{
    ConnectionStatus, Message, COLOR_ACCENT, COLOR_SURFACE, COLOR_TEXT_DIM, ICON_INFO, ICON_TRASH,
    NERD_FONT,
};
use iced::widget::{button, checkbox, column, container, pick_list, row, slider, text, text_input};
use iced::{Alignment, Border, Element, Length};

impl KuVpnGui {
    pub fn view_advanced_settings(&self) -> Element<'_, Message> {
        let is_locked = self.status != ConnectionStatus::Disconnected;

        let locked_hint = if is_locked {
            container(
                row![
                    text(ICON_INFO).font(NERD_FONT).color(COLOR_ACCENT),
                    text("Settings locked during active session.")
                        .size(11)
                        .color(COLOR_TEXT_DIM),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding(5)
        } else {
            container(iced::widget::Space::new().height(0))
        };

        container(
            column![
                row![
                    text("CONFIGURATION")
                        .size(14)
                        .color(COLOR_ACCENT)
                        .font(NERD_FONT)
                        .width(Length::Fill),
                    locked_hint,
                ]
                .align_y(Alignment::Center),
                // Parity with CLI options
                self.view_field(
                    "Gateway URL",
                    &self.settings.url,
                    is_locked,
                    Message::UrlChanged
                ),
                self.view_field(
                    "DSID Domain",
                    &self.settings.domain,
                    is_locked,
                    Message::DomainChanged
                ),
                self.view_field(
                    "Login Email",
                    &self.settings.email,
                    is_locked,
                    Message::EmailChanged
                ),
                row![
                    text("OC Path:").width(Length::Fixed(120.0)),
                    text_input("openconnect", &self.settings.openconnect_path)
                        .on_input(if is_locked {
                            |_| Message::Tick
                        } else {
                            Message::OpenConnectPathChanged
                        })
                        .padding(8),
                    button(
                        text(if self.oc_test_result == Some(true) {
                            "✓"
                        } else if self.oc_test_result == Some(false) {
                            "✗"
                        } else {
                            "Test"
                        })
                        .font(NERD_FONT)
                    )
                    .on_press(if is_locked {
                        Message::Tick
                    } else {
                        Message::TestOpenConnect
                    })
                    .style(if self.oc_test_result == Some(true) {
                        button::success
                    } else if self.oc_test_result == Some(false) {
                        button::danger
                    } else {
                        button::secondary
                    }),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![
                    text("Log Level:").width(Length::Fixed(120.0)),
                    slider(
                        0.0..=5.0,
                        self.settings.log_level_val,
                        if is_locked {
                            |_| Message::Tick
                        } else {
                            Message::LogLevelSliderChanged
                        }
                    )
                    .step(1.0)
                    .width(Length::Fill),
                    text(match self.settings.log_level_val.round() as i32 {
                        0 => "Off",
                        1 => "Error",
                        2 => "Warn",
                        3 => "Info",
                        4 => "Debug",
                        5 => "Trace",
                        _ => "Info",
                    })
                    .width(Length::Fixed(60.0)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                {
                    #[cfg(not(windows))]
                    {
                        row![
                            text("Elevation:").width(Length::Fixed(120.0)),
                            pick_list(
                                vec!["pkexec".to_string(), "sudo".to_string(), "doas".to_string()],
                                Some(self.settings.escalation_tool.clone()),
                                if is_locked {
                                    |_| Message::Tick
                                } else {
                                    Message::EscalationToolChanged
                                }
                            )
                            .width(Length::Fill),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center)
                    }
                    #[cfg(windows)]
                    {
                        iced::widget::space::Space::new(Length::Shrink, Length::Shrink)
                    }
                },
                column![
                    row![
                        text("Login Mode:").width(Length::Fixed(120.0)),
                        slider(
                            0.0..=2.0,
                            self.settings.login_mode_val,
                            if is_locked {
                                |_| Message::Tick
                            } else {
                                Message::LoginModeChanged
                            }
                        )
                        .step(1.0)
                        .width(Length::Fill),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    text(match self.settings.login_mode_val.round() as i32 {
                        0 => "Full Automatic (Headless + Auto-Login)",
                        1 => "Visual Automatic (Browser + Auto-Login)",
                        _ => "Manual Mode (Browser + Manual Entry)",
                    })
                    .size(12)
                    .color(COLOR_TEXT_DIM)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
                ]
                .spacing(5),
                row![
                    checkbox(self.settings.close_to_tray).on_toggle(Message::CloseToTrayToggled),
                    text("Close window to system tray"),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![
                    button(
                        row![
                            text(ICON_TRASH).font(NERD_FONT),
                            text("WIPE SAVED SESSION").font(NERD_FONT).size(12),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center)
                    )
                    .padding(10)
                    .on_press(Message::ClearSessionPressed)
                    .style(button::secondary),
                    button(
                        row![
                            text("\u{f021}").font(NERD_FONT), // Refresh icon for reset
                            text("RESET TO DEFAULTS").font(NERD_FONT).size(12),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center)
                    )
                    .padding(10)
                    .on_press(if is_locked {
                        Message::Tick
                    } else {
                        Message::ResetSettings
                    })
                    .style(button::secondary),
                ]
                .spacing(10)
            ]
            .spacing(12),
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

    fn view_field(
        &self,
        label: &str,
        value: &str,
        locked: bool,
        on_change: fn(String) -> Message,
    ) -> Element<'_, Message> {
        row![
            text(format!("{}:", label)).width(Length::Fixed(120.0)),
            text_input(label, value)
                .on_input(if locked { |_| Message::Tick } else { on_change })
                .padding(8),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }
}
