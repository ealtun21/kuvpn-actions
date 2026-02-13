use crate::app::KuVpnGui;
use crate::types::{ConnectionStatus, Message, KU_LOGO_BYTES, ICON_POWER_SVG};
use iced::widget::{button, row, svg, text};
use iced::{Alignment, Element, Length};

impl KuVpnGui {
    pub fn view_actions(&self) -> Element<'_, Message> {
        match self.status {
            ConnectionStatus::Disconnected | ConnectionStatus::Error => button(
                row![
                    svg(svg::Handle::from_memory(KU_LOGO_BYTES))
                        .width(20)
                        .height(20)
                        .style(|_, _| svg::Style { color: Some(iced::Color::WHITE) }),
                    text("JOIN NETWORK").size(16),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding(15)
            .width(Length::Fixed(220.0))
            .on_press(Message::ConnectPressed)
            .style(button::primary)
            .into(),
            _ => {
                button(
                    row![
                        svg(svg::Handle::from_memory(ICON_POWER_SVG))
                            .width(16)
                            .height(16)
                            .style(|_, _| svg::Style { color: Some(crate::types::COLOR_TEXT) }),
                        text(if self.status == ConnectionStatus::Connecting {
                            "CANCEL"
                        } else {
                            "DISCONNECT"
                        })
                        .size(16),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding(15)
                .width(Length::Fixed(220.0))
                .on_press(Message::DisconnectPressed)
                .style(button::secondary)
                .into()
            }
        }
    }
}