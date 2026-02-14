use crate::app::KuVpnGui;
use crate::types::{
    card, Message, COLOR_SUCCESS, COLOR_TEXT_DIM, COLOR_WARNING, ICON_REFRESH_SVG,
    ICON_SHIELD_CHECK_SVG, ICON_SHIELD_SVG,
};
use iced::widget::{column, container, row, svg, text};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector};
use kuvpn::ConnectionStatus;

impl KuVpnGui {
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
            ConnectionStatus::Error => (
                Color::from_rgb(0.8, 0.2, 0.2),
                ICON_SHIELD_SVG,
                "Connection Error",
            ),
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

        let mut content = column![
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
                .size(17)
                .color(color)
                .align_x(iced::alignment::Horizontal::Center)
                .width(Length::Fill),
            container(
                iced::widget::scrollable(
                    text(match self.status {
                        ConnectionStatus::Connected => "Internal Resources Available".to_string(),
                        ConnectionStatus::Error => {
                            "Check Console for details. Try switching to Manual mode in Settings.".to_string()
                        }
                        ConnectionStatus::Connecting | ConnectionStatus::Disconnecting => {
                            self.status_message.clone()
                        }
                        _ => "Koç University Access Restricted".to_string(),
                    })
                    .size(12)
                    .color(COLOR_TEXT_DIM)
                    .align_x(iced::alignment::Horizontal::Center)
                    .wrapping(iced::widget::text::Wrapping::Word)
                )
                .height(100)
            )
            .width(Length::Fill)
            .max_width(480.0)
            .center_x(Length::Fill),
        ]
        .spacing(10)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .max_width(480.0);

        // Add connection details when connected
        if self.status == ConnectionStatus::Connected {
            if let Some(start_time) = self.connection_start {
                let elapsed = start_time.elapsed();
                let duration_str = format!(
                    "{}h {}m {}s",
                    elapsed.as_secs() / 3600,
                    (elapsed.as_secs() % 3600) / 60,
                    elapsed.as_secs() % 60
                );

                let mut status_col = column![row![
                    text("Duration:").size(12).width(Length::Shrink).color(COLOR_TEXT_DIM),
                    text(duration_str).size(13).color(COLOR_SUCCESS),
                ]
                .spacing(6)
                .align_y(Alignment::Center)];

                #[cfg(unix)]
                {
                    status_col = status_col.push(
                        row![
                            text("Interface:")
                                .size(12)
                                .width(Length::Shrink)
                                .color(COLOR_TEXT_DIM),
                            text("kuvpn0").size(13).color(COLOR_SUCCESS),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    );
                }

                content = content.push(
                    container(status_col.spacing(10))
                        .padding(18)
                        .width(Length::Shrink)
                        .style(card),
                );
            }
        }

        content.into()
    }
}
