use iced::widget::{button, row, svg, text};
use iced::{Alignment, Element, Length};
use crate::app::KuVpnGui;
use crate::types::{ConnectionStatus, Message, KU_LOGO_BYTES, NERD_FONT};

impl KuVpnGui {
    pub fn view_actions(&self) -> Element<'_, Message> {
        match self.status {
            ConnectionStatus::Disconnected => {
                button(
                    row![
                        svg(svg::Handle::from_memory(KU_LOGO_BYTES)).width(20).height(20),
                        text("JOIN NETWORK").font(NERD_FONT).size(16),
                    ].spacing(10).align_y(Alignment::Center)
                )
                .padding(15)
                .width(Length::Fixed(220.0))
                .on_press(Message::ConnectPressed)
                .style(button::primary)
                .into()
            }
            _ => {
                button(
                    row![
                        text("\u{f011}").font(NERD_FONT), // Power icon
                        text(if self.status == ConnectionStatus::Connecting { "CANCEL" } else { "DISCONNECT" })
                            .font(NERD_FONT).size(16),
                    ].spacing(10).align_y(Alignment::Center)
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
