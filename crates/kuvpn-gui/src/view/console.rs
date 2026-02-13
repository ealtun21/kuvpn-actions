use crate::app::KuVpnGui;
use crate::types::{COLOR_TEXT_DIM, ICON_TERMINAL_SVG, Message};
use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Border, Color, Element, Font, Length};

impl KuVpnGui {
    pub fn view_console(&self) -> Element<'_, Message> {
        container(
            column![
                row![
                    svg(svg::Handle::from_memory(ICON_TERMINAL_SVG))
                        .width(14)
                        .height(14)
                        .style(|_, _| svg::Style { color: Some(COLOR_TEXT_DIM) }),
                    text("SESSION LOGS")
                        .size(12)
                        .color(COLOR_TEXT_DIM),
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