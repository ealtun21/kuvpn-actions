use crate::app::KuVpnGui;
use crate::types::{
    COLOR_SUCCESS, COLOR_TEXT_DIM, COLOR_WARNING, ICON_REFRESH_SVG, ICON_SHIELD_CHECK_SVG,
    ICON_SHIELD_SVG, Message,
};
use iced::widget::{column, container, svg, text};
use iced::{Alignment, Border, Color, Element};
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

        let mut icon_display = svg(svg::Handle::from_memory(icon_svg))
            .width(80)
            .height(80)
            .style(move |_, _| svg::Style { color: Some(color) });

        if self.status == ConnectionStatus::Connecting || self.status == ConnectionStatus::Disconnecting {
            icon_display = icon_display.rotation(self.rotation);
        }

        column![
            container(icon_display)
                .center_x(180)
                .center_y(180)
                .style(move |_| container::Style {
                    border: Border {
                        color: color,
                        width: 2.0,
                        radius: 90.0.into(),
                    },
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
        .align_x(Alignment::Center)
        .into()
    }
}
