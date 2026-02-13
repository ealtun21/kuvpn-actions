use crate::app::KuVpnGui;
use crate::types::{
    COLOR_ACCENT, COLOR_SURFACE, COLOR_TEXT_DIM, ICON_INFO_SVG, ICON_REFRESH_SVG, ICON_TRASH_SVG,
    Message, btn_secondary, log_level_name, login_mode_name,
};
#[cfg(not(windows))]
use iced::widget::pick_list;
use iced::widget::{
    button, checkbox, column, container, row, slider, svg, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Length};
use kuvpn::ConnectionStatus;

impl KuVpnGui {
    pub fn view_advanced_settings(&self) -> Element<'_, Message> {
        let is_locked = self.status != ConnectionStatus::Disconnected;

        let locked_hint = if is_locked {
            container(
                row![
                    svg(svg::Handle::from_memory(ICON_INFO_SVG))
                        .width(14)
                        .height(14)
                        .style(|_, _| svg::Style { color: Some(COLOR_ACCENT) }),
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

        let section_label = |label: &'static str| -> Element<'_, Message> {
            text(label)
                .size(11)
                .color(Color::from_rgb(0.40, 0.40, 0.40))
                .into()
        };

        let divider = || -> Element<'_, Message> {
            container(iced::widget::Space::new().height(0))
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.20, 0.20, 0.20).into()),
                    ..Default::default()
                })
                .into()
        };

        container(
            column![
                // Header
                row![
                    text("CONFIGURATION")
                        .size(14)
                        .color(COLOR_ACCENT)
                        .width(Length::Fill),
                    locked_hint,
                ]
                .align_y(Alignment::Center),
                // --- Network Section ---
                section_label("NETWORK"),
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
                divider(),
                // --- Browser Section ---
                section_label("BROWSER"),
                self.view_field(
                    "Login Email",
                    &self.settings.email,
                    is_locked,
                    Message::EmailChanged
                ),
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
                    text(login_mode_name(self.settings.login_mode_val))
                    .size(12)
                    .color(COLOR_TEXT_DIM)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
                ]
                .spacing(5),
                divider(),
                // --- System Section ---
                section_label("SYSTEM"),
                row![
                    text("Log Level:").width(Length::Fixed(120.0)),
                    slider(
                        0.0..=5.0,
                        self.settings.log_level_val,
                        Message::LogLevelSliderChanged
                    )
                    .step(1.0)
                    .width(Length::Fill),
                    text(log_level_name(self.settings.log_level_val))
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
                        iced::widget::Space::new()
                            .width(Length::Shrink)
                            .height(Length::Shrink)
                    }
                },
                row![
                    checkbox(self.settings.close_to_tray).on_toggle(Message::CloseToTrayToggled),
                    text("Close window to system tray"),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                divider(),
                // --- Actions Section ---
                section_label("ACTIONS"),
                row![
                    button(
                        row![
                            svg(svg::Handle::from_memory(ICON_TRASH_SVG))
                                .width(14)
                                .height(14)
                                .style(|_, _| svg::Style { color: Some(COLOR_TEXT_DIM) }),
                            text("WIPE SAVED SESSION").size(12),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center)
                    )
                    .padding(10)
                    .on_press(Message::ClearSessionPressed)
                    .style(btn_secondary),
                    button(
                        row![
                            svg(svg::Handle::from_memory(ICON_REFRESH_SVG))
                                .width(14)
                                .height(14)
                                .style(|_, _| svg::Style { color: Some(COLOR_TEXT_DIM) }),
                            text("RESET TO DEFAULTS").size(12),
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
                    .style(btn_secondary),
                ]
                .spacing(10)
            ]
            .spacing(10),
        )
        .padding(25)
        .style(|_| container::Style {
            background: Some(COLOR_SURFACE.into()),
            border: Border {
                color: Color::from_rgb(0.15, 0.15, 0.15),
                width: 1.0,
                radius: 8.0.into(),
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
