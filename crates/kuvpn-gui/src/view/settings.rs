use crate::app::KuVpnGui;
use crate::types::{
    COLOR_TEXT, COLOR_TEXT_DIM, ICON_INFO_SVG, ICON_REFRESH_SVG, ICON_TRASH_SVG,
    Message, btn_secondary, card, btn_segment_selected, btn_segment_unselected, SegmentPosition, toggler_neutral,
};
use iced::widget::{
    button, column, container, row, svg, text, text_input, toggler,
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
                        .style(|_, _| svg::Style { color: Some(COLOR_TEXT_DIM) }),
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
                        .color(COLOR_TEXT)
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
                    text("OC Path:").size(11).width(Length::Fixed(120.0)),
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
                row![
                    text("Login Mode:").size(11).width(Length::Fixed(120.0)),
                    self.view_segmented_control(
                        &["Full Auto", "Visual Auto", "Manual"],
                        &[0.0, 1.0, 2.0],
                        self.settings.login_mode_val,
                        is_locked,
                        Message::LoginModeChanged
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                divider(),
                // --- System Section ---
                section_label("SYSTEM"),
                row![
                    text("Log Level:").size(11).width(Length::Fixed(120.0)),
                    self.view_segmented_control(
                        &["Off", "Error", "Warn", "Info", "Debug", "Trace"],
                        &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
                        self.settings.log_level_val,
                        false,
                        Message::LogLevelSliderChanged
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                {
                    #[cfg(not(windows))]
                    {
                        row![
                            text("Elevation:").size(11).width(Length::Fixed(120.0)),
                            self.view_segmented_control_str(
                                &["pkexec", "sudo", "doas"],
                                &self.settings.escalation_tool,
                                is_locked,
                                Message::EscalationToolChanged
                            ),
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
                    toggler(self.settings.close_to_tray)
                        .on_toggle(Message::CloseToTrayToggled)
                        .style(toggler_neutral),
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
        .style(card)
        .into()
    }

    fn view_segmented_control<'a>(
        &self,
        options: &'a [&'static str],
        values: &'a [f32],
        current_value: f32,
        locked: bool,
        on_change: fn(f32) -> Message,
    ) -> Element<'a, Message> {
        let buttons: Vec<Element<'a, Message>> = options
            .iter()
            .zip(values.iter())
            .enumerate()
            .map(|(idx, (label, &value))| {
                let is_selected = (current_value - value).abs() < 0.1;

                let position = if options.len() == 1 {
                    SegmentPosition::Single
                } else if idx == 0 {
                    SegmentPosition::Left
                } else if idx == options.len() - 1 {
                    SegmentPosition::Right
                } else {
                    SegmentPosition::Middle
                };

                button(text(*label).size(12))
                    .padding([8, 12])
                    .on_press(if locked { Message::Tick } else { on_change(value) })
                    .style(move |theme, status| {
                        if is_selected {
                            btn_segment_selected(theme, status, position)
                        } else {
                            btn_segment_unselected(theme, status, position)
                        }
                    })
                    .into()
            })
            .collect();

        row(buttons).spacing(-1.0).into()
    }

    fn view_segmented_control_str<'a>(
        &self,
        options: &'a [&'static str],
        current_value: &str,
        locked: bool,
        on_change: fn(String) -> Message,
    ) -> Element<'a, Message> {
        let buttons: Vec<Element<'a, Message>> = options
            .iter()
            .enumerate()
            .map(|(idx, &label)| {
                let is_selected = current_value == label;

                let position = if options.len() == 1 {
                    SegmentPosition::Single
                } else if idx == 0 {
                    SegmentPosition::Left
                } else if idx == options.len() - 1 {
                    SegmentPosition::Right
                } else {
                    SegmentPosition::Middle
                };

                button(text(label).size(12))
                    .padding([8, 12])
                    .on_press(if locked { Message::Tick } else { on_change(label.to_string()) })
                    .style(move |theme, status| {
                        if is_selected {
                            btn_segment_selected(theme, status, position)
                        } else {
                            btn_segment_unselected(theme, status, position)
                        }
                    })
                    .into()
            })
            .collect();

        row(buttons).spacing(-1.0).into()
    }

    fn view_field<'a>(
        &self,
        label: &'a str,
        value: &'a str,
        locked: bool,
        on_change: fn(String) -> Message,
    ) -> Element<'a, Message> {
        column![
            text(label)
                .size(11)
                .color(COLOR_TEXT),
            text_input("", value)
                .on_input(if locked { |_| Message::Tick } else { on_change })
                .padding(12)
                .style(move |_theme, status| {
                    let mut style = text_input::default(_theme, status);
                    style.background = iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.08));
                    style.border = Border {
                        color: match status {
                            text_input::Status::Active => Color::from_rgb(0.20, 0.20, 0.20),
                            text_input::Status::Focused { is_hovered } => {
                                if is_hovered {
                                    Color::from_rgb(0.35, 0.35, 0.35)
                                } else {
                                    Color::from_rgb(0.30, 0.30, 0.30)
                                }
                            }
                            text_input::Status::Hovered => Color::from_rgb(0.25, 0.25, 0.25),
                            text_input::Status::Disabled => Color::from_rgb(0.15, 0.15, 0.15),
                        },
                        width: 1.0,
                        radius: 8.0.into(),
                    };
                    style
                }),
        ]
        .spacing(6)
        .into()
    }
}
