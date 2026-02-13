use crate::app::KuVpnGui;
use crate::types::{Message, ICON_SETTINGS_SVG, ICON_TERMINAL_SVG, KU_LOGO_BYTES};
use iced::widget::{button, row, svg, text};
use iced::{Alignment, Element, Length};

impl KuVpnGui {
    pub fn view_header(&self) -> Element<'_, Message> {
        row![
            svg(svg::Handle::from_memory(KU_LOGO_BYTES))
                .width(32)
                .height(32)
                .style(|_, _| svg::Style { color: Some(iced::Color::WHITE) }),
            text("KUVPN").size(24).width(Length::Fill),
            button(
                svg(svg::Handle::from_memory(ICON_SETTINGS_SVG))
                    .width(24)
                    .height(24)
                    .style(|_, _| svg::Style { color: Some(crate::types::COLOR_TEXT) })
            )
            .on_press(Message::ToggleAdvanced)
            .style(button::text),
            button(
                svg(svg::Handle::from_memory(ICON_TERMINAL_SVG))
                    .width(24)
                    .height(24)
                    .style(|_, _| svg::Style { color: Some(crate::types::COLOR_TEXT) })
            )
            .on_press(Message::ToggleConsole)
            .style(button::text),
        ]
        .spacing(15)
        .align_y(Alignment::Center)
        .into()
    }
}
