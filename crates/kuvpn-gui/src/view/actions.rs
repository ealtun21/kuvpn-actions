use crate::app::KuVpnGui;
use crate::types::{
    btn_danger, btn_primary, ConnectionStatus, Message, ICON_POWER_SVG, KU_LOGO_BYTES,
};
use iced::widget::{button, container, row, svg, text};
use iced::{Alignment, Element, Length};

impl KuVpnGui {
    pub fn view_actions(&self) -> Element<'_, Message> {
        match self.status {
            ConnectionStatus::Disconnected | ConnectionStatus::Error => button(
                container(
                    row![
                        svg(svg::Handle::from_memory(KU_LOGO_BYTES))
                            .width(20)
                            .height(20)
                            .style(|_, _| svg::Style {
                                color: Some(iced::Color::WHITE)
                            }),
                        text("JOIN NETWORK").size(15).color(iced::Color::WHITE),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .center_x(Length::Fill),
            )
            .padding([14, 20])
            .width(Length::Fill)
            .on_press(Message::ConnectPressed)
            .style(btn_primary)
            .into(),
            _ => button(
                container(
                    row![
                        svg(svg::Handle::from_memory(ICON_POWER_SVG))
                            .width(16)
                            .height(16)
                            .style(|_, _| svg::Style {
                                color: Some(iced::Color::WHITE)
                            }),
                        text(if self.status == ConnectionStatus::Connecting {
                            "CANCEL"
                        } else {
                            "DISCONNECT"
                        })
                        .size(15)
                        .color(iced::Color::WHITE),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .center_x(Length::Fill),
            )
            .padding([14, 20])
            .width(Length::Fill)
            .on_press(Message::DisconnectPressed)
            .style(btn_danger)
            .into(),
        }
    }
}
