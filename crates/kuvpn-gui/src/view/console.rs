use crate::app::KuVpnGui;
use crate::types::{Message, COLOR_TEXT_DIM, ICON_TERMINAL, NERD_FONT};
use iced::widget::{column, container, row, scrollable, text};
use iced::{Border, Color, Element, Font, Length};

impl KuVpnGui {
    pub fn view_console(&self) -> Element<'_, Message> {
        container(
            column![
                row![
                    text(ICON_TERMINAL).font(NERD_FONT).color(COLOR_TEXT_DIM),
                    text("SESSION LOGS")
                        .size(12)
                        .color(COLOR_TEXT_DIM)
                        .font(NERD_FONT),
                ]
                .spacing(10),
                scrollable(
                    text(self.logs.join("\n"))
                        .font(Font::MONOSPACE)
                        .size(11)
                        .color(COLOR_TEXT_DIM),
                )
                .height(Length::Fixed(140.0)),
            ]
            .spacing(10),
        )
        .padding(15)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.04, 0.04, 0.04).into()),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }
}
