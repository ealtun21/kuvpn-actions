use crate::app::KuVpnGui;
use crate::types::{
    ConnectionStatus, Message, COLOR_SUCCESS, COLOR_TEXT_DIM, COLOR_WARNING, ICON_REFRESH,
    ICON_SHIELD, ICON_SHIELD_CHECK, NERD_FONT,
};
use iced::widget::{column, container, text};
use iced::{Alignment, Border, Element, Color};

impl KuVpnGui {
    pub fn view_status_circle(&self) -> Element<'_, Message> {
        let (color, icon, status_text) = match self.status {
            ConnectionStatus::Disconnected => (COLOR_TEXT_DIM, ICON_SHIELD, "Public Access"),
            ConnectionStatus::Connecting => (COLOR_WARNING, ICON_REFRESH, "Joining Campus..."),
            ConnectionStatus::Connected => (COLOR_SUCCESS, ICON_SHIELD_CHECK, "KU Network Active"),
            ConnectionStatus::Disconnecting => (COLOR_WARNING, ICON_REFRESH, "Disconnecting..."),
            ConnectionStatus::Error => (Color::from_rgb(0.8, 0.2, 0.2), ICON_SHIELD, "Connection Error"),
        };

        let icon_display = text(icon).font(NERD_FONT).size(80).color(color);

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
            text(status_text).size(18).font(NERD_FONT).color(color),
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
