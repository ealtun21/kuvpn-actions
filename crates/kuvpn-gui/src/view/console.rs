use crate::app::KuVpnGui;
use crate::theme::Palette;
use crate::types::Message;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Font, Length, Padding};

fn log_line_color(line: &str, p: Palette) -> Color {
    if line.starts_with("[ERR]") || line.starts_with("[!]") {
        p.danger
    } else if line.starts_with("[WRN]") {
        p.warning
    } else if line.starts_with("[INF]") || line.starts_with("[*]") {
        p.text_muted
    } else if line.starts_with("[DBG]") || line.starts_with("[TRC]") {
        p.text_disabled
    } else {
        p.text_muted
    }
}

impl KuVpnGui {
    pub fn view_console(&self) -> Element<'_, Message> {
        let s = self.styler();
        let p = s.p;

        let log_lines: Vec<Element<'_, Message>> = self
            .logs
            .iter()
            .map(|line| {
                text(line.as_str())
                    .font(Font::MONOSPACE)
                    .size(12)
                    .color(log_line_color(line, p))
                    .into()
            })
            .collect();

        container(
            column![
                row![
                    text("Session Logs")
                        .size(14)
                        .color(p.text)
                        .width(Length::Fill),
                    button(text("Copy All").size(11).color(p.text))
                        .padding([8, 14])
                        .on_press(Message::CopyLogs)
                        .style(s.btn_secondary()),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                scrollable(
                    container(column(log_lines).spacing(2))
                        .width(Length::Fill)
                        .padding(Padding {
                            top: 0.0,
                            right: 24.0,
                            bottom: 0.0,
                            left: 0.0,
                        }),
                )
                .height(Length::Fill)
                .direction(scrollable::Direction::Both {
                    vertical: scrollable::Scrollbar::default(),
                    horizontal: scrollable::Scrollbar::default(),
                })
                .style(s.scrollbar()),
            ]
            .spacing(12),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(s.card())
        .into()
    }
}
