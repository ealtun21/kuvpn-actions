use crate::app::KuVpnGui;
use crate::types::{
    COLOR_SUCCESS, COLOR_TEXT_DIM, COLOR_WARNING, ICON_REFRESH_SVG, ICON_SHIELD_CHECK_SVG,
    ICON_SHIELD_SVG, Message, card,
};
use iced::widget::{column, container, row, svg, text};
use iced::{Alignment, Border, Color, Element, Shadow, Vector};
use kuvpn::ConnectionStatus;

impl KuVpnGui {
    pub fn view_status_circle(&self) -> Element<'_, Message> {
        let (color, icon_svg, status_text) = match self.status {
            ConnectionStatus::Disconnected => {
                (COLOR_TEXT_DIM, ICON_SHIELD_SVG, "Public Access")
            }
            ConnectionStatus::Connecting => {
                (COLOR_WARNING, ICON_REFRESH_SVG, "Joining Campus...")
            }
            ConnectionStatus::Connected => (
                COLOR_SUCCESS,
                ICON_SHIELD_CHECK_SVG,
                "KU Network Active",
            ),
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
            .width(80)
            .height(80)
            .style(move |_, _| svg::Style { color: Some(color) });

        if self.status == ConnectionStatus::Connecting || self.status == ConnectionStatus::Disconnecting {
            icon_display = icon_display.rotation(self.rotation);
        }

        let mut content = column![
            container(icon_display)
                .center_x(180)
                .center_y(180)
                .style(move |_| container::Style {
                    border: Border {
                        color,
                        width: 2.5,
                        radius: 90.0.into(),
                    },
                    shadow: glow,
                    ..Default::default()
                }),
            text(status_text).size(18).color(color),
            text(match self.status {
                ConnectionStatus::Connected => "Internal Resources Available",
                ConnectionStatus::Error => "Something went wrong. Check logs.",
                _ => "Koç University Access Restricted",
            })
            .size(12)
            .color(COLOR_TEXT_DIM),
        ]
        .spacing(15)
        .align_x(Alignment::Center);

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

                content = content.push(
                    container(
                        column![
                            row![
                                text("Duration:").width(80),
                                text(duration_str).color(COLOR_SUCCESS),
                            ]
                            .spacing(10),
                            row![
                                text("Interface:").width(80),
                                text("kuvpn0").color(COLOR_SUCCESS),
                            ]
                            .spacing(10),
                        ]
                        .spacing(8),
                    )
                    .padding(15)
                    .style(card),
                );
            }
        }

        content.into()
    }
}
