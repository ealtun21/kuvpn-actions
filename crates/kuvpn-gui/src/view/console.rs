use crate::app::KuVpnGui;
use crate::types::{COLOR_TEXT_DIM, COLOR_WARNING, ICON_TERMINAL_SVG, Message};
use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Border, Color, Element, Font, Length};

fn log_line_color(line: &str) -> Color {
    if line.starts_with("[ERR]") || line.starts_with("[!]") {
        Color::from_rgb(0.85, 0.25, 0.25)
    } else if line.starts_with("[WRN]") {
        COLOR_WARNING
    } else if line.starts_with("[INF]") || line.starts_with("[*]") {
        COLOR_TEXT_DIM
    } else if line.starts_with("[DBG]") || line.starts_with("[TRC]") {
        Color::from_rgb(0.35, 0.35, 0.35)
    } else {
        COLOR_TEXT_DIM
    }
}

impl KuVpnGui {
    pub fn view_console(&self) -> Element<'_, Message> {
        let log_lines: Vec<Element<'_, Message>> = self
            .logs
            .iter()
            .map(|line| {
                text(line.as_str())
                    .font(Font::MONOSPACE)
                    .size(12)
                    .color(log_line_color(line))
                    .into()
            })
            .collect();

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
                    column(log_lines).spacing(2),
                )
                .height(Length::Fixed(160.0)),
            ]
            .spacing(10),
        )
        .padding(15)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.04, 0.04, 0.04).into()),
            border: Border {
                color: Color::from_rgb(0.15, 0.15, 0.15),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
    }
}
