use crate::app::KuVpnGui;
use crate::types::{Message, KU_LOGO_BYTES};
use iced::widget::{row, svg, text};
use iced::{Alignment, Element, Length};

impl KuVpnGui {
    pub fn view_header(&self) -> Element<'_, Message> {
        row![
            svg(svg::Handle::from_memory(KU_LOGO_BYTES))
                .width(36)
                .height(36)
                .style(|_, _| svg::Style {
                    color: Some(iced::Color::WHITE)
                }),
            text("KUVPN")
                .size(24)
                .width(Length::Fill)
                .color(iced::Color::WHITE),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    }
}
