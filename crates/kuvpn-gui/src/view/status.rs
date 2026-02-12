use iced::widget::{column, container, text};
use iced::{Alignment, Border, Element};
use crate::app::KuVpnGui;
use crate::types::{
    ConnectionStatus, Message, COLOR_SUCCESS, COLOR_TEXT_DIM, COLOR_WARNING,
    ICON_REFRESH, ICON_SHIELD, ICON_SHIELD_CHECK, NERD_FONT,
};

impl KuVpnGui {
    pub fn view_status_circle(&self) -> Element<'_, Message> {
        let (color, icon, status_text) = match self.status {
            ConnectionStatus::Disconnected => (COLOR_TEXT_DIM, ICON_SHIELD, "Public Access"),
            ConnectionStatus::Connecting => (COLOR_WARNING, ICON_REFRESH, "Joining Campus..."),
            ConnectionStatus::Connected => (COLOR_SUCCESS, ICON_SHIELD_CHECK, "KU Network Active"),
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
            text(status_text)
                .size(18)
                .font(NERD_FONT)
                .color(color),
            text(if self.status == ConnectionStatus::Connected { "Internal Resources Available" } else { "Koç University Access Restricted" })
                .size(12)
                .color(COLOR_TEXT_DIM)
        ]
        .spacing(15)
        .align_x(Alignment::Center)
        .into()
    }
}
