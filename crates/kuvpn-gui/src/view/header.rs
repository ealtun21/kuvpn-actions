use iced::widget::{button, row, svg, text};
use iced::{Alignment, Element, Length};
use crate::app::KuVpnGui;
use crate::types::{Message, ICON_SETTINGS, ICON_TERMINAL, KU_LOGO_BYTES, NERD_FONT};

impl KuVpnGui {
    pub fn view_header(&self) -> Element<'_, Message> {
        row![
            svg(svg::Handle::from_memory(KU_LOGO_BYTES)).width(32).height(32),
            text("KUVPN").size(24).font(NERD_FONT).width(Length::Fill),
            button(text(ICON_SETTINGS).font(NERD_FONT).size(24))
                .on_press(Message::ToggleAdvanced)
                .style(button::text),
            button(text(ICON_TERMINAL).font(NERD_FONT).size(24))
                .on_press(Message::ToggleConsole)
                .style(button::text),
        ]
        .spacing(15)
        .align_y(Alignment::Center)
        .into()
    }
}
