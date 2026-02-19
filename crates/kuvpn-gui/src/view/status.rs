use crate::app::KuVpnGui;
use crate::types::{
    Message, COLOR_SUCCESS, COLOR_TEXT, COLOR_TEXT_DIM, COLOR_WARNING, ICON_CLOCK_SVG,
    ICON_INFO_SVG, ICON_PHONE_SVG, ICON_REFRESH_SVG, ICON_SHIELD_CHECK_SVG,
    ICON_SHIELD_SVG, ICON_SHIELD_X_SVG,
};
use iced::widget::{column, container, row, svg, text};
use iced::{Alignment, Border, Color, Element, Font, Length, Shadow, Vector};
use kuvpn::ConnectionStatus;

impl KuVpnGui {
    /// Hero status area: circle + status text + subtitle
    pub fn view_status_circle(&self) -> Element<'_, Message> {
        let (color, icon_svg, status_text) = match self.status {
            ConnectionStatus::Disconnected => (COLOR_TEXT_DIM, ICON_SHIELD_SVG, "Public Access"),
            ConnectionStatus::Connecting => (COLOR_WARNING, ICON_REFRESH_SVG, "Joining Campus..."),
            ConnectionStatus::Connected => {
                (COLOR_SUCCESS, ICON_SHIELD_CHECK_SVG, "KU Network Active")
            }
            ConnectionStatus::Disconnecting => {
                (COLOR_WARNING, ICON_REFRESH_SVG, "Disconnecting...")
            }
            ConnectionStatus::Error => {
                let error_type = match self.error_category {
                    Some(kuvpn::ErrorCategory::Authentication) => "Authentication Error",
                    Some(kuvpn::ErrorCategory::Connection) => "Connection Error",
                    Some(kuvpn::ErrorCategory::System) => "System Error",
                    None => "Error",
                };
                (Color::from_rgb(0.8, 0.2, 0.2), ICON_SHIELD_X_SVG, error_type)
            }
        };

        let glow = match self.status {
            ConnectionStatus::Disconnected => Shadow {
                color: Color::TRANSPARENT,
                offset: Vector::ZERO,
                blur_radius: 0.0,
            },
            ConnectionStatus::Connected => Shadow {
                color: Color::from_rgba(0.42, 0.55, 0.35, 0.4),
                offset: Vector::ZERO,
                blur_radius: 25.0,
            },
            ConnectionStatus::Connecting | ConnectionStatus::Disconnecting => Shadow {
                color: Color::from_rgba(0.80, 0.60, 0.30, 0.35),
                offset: Vector::ZERO,
                blur_radius: 20.0,
            },
            ConnectionStatus::Error => Shadow {
                color: Color::from_rgba(0.8, 0.2, 0.2, 0.35),
                offset: Vector::ZERO,
                blur_radius: 20.0,
            },
        };

        let mut icon_display = svg(svg::Handle::from_memory(icon_svg))
            .width(60)
            .height(60)
            .style(move |_, _| svg::Style { color: Some(color) });

        if self.status == ConnectionStatus::Connecting
            || self.status == ConnectionStatus::Disconnecting
        {
            icon_display = icon_display.rotation(self.rotation);
        }

        let subtitle = match self.status {
            ConnectionStatus::Connected => "Internal Resources Available".to_string(),
            ConnectionStatus::Error => {
                if self.automation_warning.is_some() {
                    "See details below".to_string()
                } else {
                    self.error_message
                        .clone()
                        .unwrap_or_else(|| "Check console for details".to_string())
                }
            }
            ConnectionStatus::Connecting | ConnectionStatus::Disconnecting => {
                self.status_message.clone()
            }
            _ => "Koc University Access Restricted".to_string(),
        };

        column![
            container(
                container(icon_display)
                    .width(Length::Fixed(140.0))
                    .height(Length::Fixed(140.0))
                    .center_x(Length::Fixed(140.0))
                    .center_y(Length::Fixed(140.0))
                    .style(move |_| container::Style {
                        border: Border {
                            color,
                            width: 2.5,
                            radius: 70.0.into(),
                        },
                        shadow: glow,
                        ..Default::default()
                    })
            )
            .width(Length::Fill)
            .center_x(Length::Fill),
            text(status_text)
                .size(18)
                .color(color)
                .align_x(iced::alignment::Horizontal::Center)
                .width(Length::Fill),
            text(subtitle)
                .size(12)
                .color(COLOR_TEXT_DIM)
                .align_x(iced::alignment::Horizontal::Center)
                .wrapping(iced::widget::text::Wrapping::Word)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
    }

    /// Connection details as inline pills
    pub fn view_connection_details(&self) -> Element<'_, Message> {
        if let Some(start_time) = self.connection_start {
            let elapsed = start_time.elapsed();
            let duration_str = format!(
                "{}h {}m {}s",
                elapsed.as_secs() / 3600,
                (elapsed.as_secs() % 3600) / 60,
                elapsed.as_secs() % 60
            );

            let pill_style = |_: &iced::Theme| container::Style {
                background: Some(Color::from_rgba(0.42, 0.55, 0.35, 0.08).into()),
                border: Border {
                    radius: 16.0.into(),
                    color: Color::from_rgba(0.42, 0.55, 0.35, 0.2),
                    width: 1.0,
                },
                ..Default::default()
            };

            let duration_pill = container(
                row![
                    svg(svg::Handle::from_memory(ICON_CLOCK_SVG))
                        .width(14)
                        .height(14)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_SUCCESS)
                        }),
                    text(duration_str).size(12).color(COLOR_SUCCESS),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 14])
            .style(pill_style);

            #[allow(unused_mut)]
            let mut details_row = row![duration_pill]
                .spacing(8)
                .align_y(Alignment::Center);

            #[cfg(unix)]
            if let Some(ref iface_name) = self.active_interface {
                let iface_display = iface_name.clone();
                let interface_pill = container(
                    row![
                        svg(svg::Handle::from_memory(crate::types::ICON_GLOBE_SVG))
                            .width(14)
                            .height(14)
                            .style(|_, _| svg::Style {
                                color: Some(COLOR_SUCCESS)
                            }),
                        text(iface_display).size(12).color(COLOR_SUCCESS),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([6, 14])
                .style(|_: &iced::Theme| container::Style {
                    background: Some(Color::from_rgba(0.42, 0.55, 0.35, 0.08).into()),
                    border: Border {
                        radius: 16.0.into(),
                        color: Color::from_rgba(0.42, 0.55, 0.35, 0.2),
                        width: 1.0,
                    },
                    ..Default::default()
                });
                details_row = details_row.push(interface_pill);
            }

            container(details_row)
                .width(Length::Fill)
                .center_x(Length::Fill)
                .into()
        } else {
            iced::widget::Space::new().height(0).into()
        }
    }

    /// MFA approval banner with phone icon and prominent code
    pub fn view_mfa_card<'a>(&'a self, code: &'a str) -> Element<'a, Message> {
        container(
            column![
                row![
                    svg(svg::Handle::from_memory(ICON_PHONE_SVG))
                        .width(22)
                        .height(22)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_WARNING)
                        }),
                    text("Approve Sign-In").size(14).color(COLOR_WARNING),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                container(
                    row![
                        text("Enter ").size(12).color(COLOR_TEXT_DIM),
                        container(
                            text(code)
                                .size(20)
                                .color(COLOR_WARNING)
                                .font(Font::MONOSPACE)
                        )
                        .padding([2, 10])
                        .style(|_| container::Style {
                            background: Some(
                                Color::from_rgba(0.80, 0.60, 0.30, 0.12).into()
                            ),
                            border: Border {
                                radius: 6.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                        text(" in Authenticator").size(12).color(COLOR_TEXT_DIM),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                )
                .width(Length::Fill)
                .center_x(Length::Fill),
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([14, 18])
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0.80, 0.60, 0.30, 0.06).into()),
            border: Border {
                color: Color::from_rgba(0.80, 0.60, 0.30, 0.3),
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    /// Warning/automation banner
    pub fn view_warning_card<'a>(&'a self, warning: &'a str) -> Element<'a, Message> {
        container(
            column![
                row![
                    svg(svg::Handle::from_memory(ICON_INFO_SVG))
                        .width(18)
                        .height(18)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_WARNING)
                        }),
                    text("Automation Issue").size(13).color(COLOR_WARNING),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text(warning)
                    .size(11)
                    .color(COLOR_TEXT)
                    .wrapping(iced::widget::text::Wrapping::Word),
            ]
            .spacing(6),
        )
        .width(Length::Fill)
        .padding([12, 16])
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0.80, 0.60, 0.30, 0.06).into()),
            border: Border {
                color: Color::from_rgba(0.80, 0.60, 0.30, 0.3),
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
    }
}
