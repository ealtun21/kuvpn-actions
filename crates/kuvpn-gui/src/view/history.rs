use crate::app::KuVpnGui;
use crate::types::{
    btn_secondary, card, custom_scrollbar, Message, COLOR_SUCCESS, COLOR_TEXT, COLOR_TEXT_DIM,
    COLOR_WARNING, ICON_CLOCK_SVG, ICON_TRASH_SVG,
};
use iced::widget::{button, column, container, row, scrollable, svg, text, Space};
use iced::{Alignment, Color, Element, Length, Padding};

impl KuVpnGui {
    pub fn view_history(&self) -> Element<'_, Message> {
        let heading = row![
            text("Connection History")
                .size(14)
                .color(COLOR_TEXT)
                .width(Length::Fill),
            button(
                row![
                    svg(svg::Handle::from_memory(ICON_TRASH_SVG))
                        .width(13)
                        .height(13)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_TEXT)
                        }),
                    text("Clear").size(11).color(COLOR_TEXT),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .on_press(Message::ClearHistory)
            .style(btn_secondary),
        ]
        .align_y(Alignment::Center);

        let events_col = if self.history.is_empty() {
            column![
                Space::new().height(Length::Fill),
                container(
                    column![
                        svg(svg::Handle::from_memory(ICON_CLOCK_SVG))
                            .width(32)
                            .height(32)
                            .style(|_, _| svg::Style {
                                color: Some(COLOR_TEXT_DIM)
                            }),
                        text("No connection history yet.")
                            .size(13)
                            .color(COLOR_TEXT_DIM),
                        text("Events are recorded when you connect or disconnect.")
                            .size(11)
                            .color(COLOR_TEXT_DIM),
                    ]
                    .spacing(8)
                    .align_x(Alignment::Center),
                )
                .center_x(Length::Fill),
                Space::new().height(Length::Fill),
            ]
            .align_x(Alignment::Center)
            .width(Length::Fill)
        } else {
            let mut col = column![].spacing(6).width(Length::Fill);
            // Show newest first
            for event in self.history.iter().rev() {
                col = col.push(view_event_row(event));
            }
            col
        };

        // Wrap events in a container with right padding so items don't render
        // under the scrollbar rail when it appears.
        let scrollable_content = container(events_col).width(Length::Fill).padding(Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 0.0,
        });

        let content = column![
            heading,
            scrollable(scrollable_content)
                .height(Length::Fill)
                .style(custom_scrollbar)
        ]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .padding(Padding {
                top: 24.0,
                right: 24.0,
                bottom: 24.0,
                left: 24.0,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(card)
            .into()
    }
}

fn view_event_row<'a>(event: &'a kuvpn::ConnectionEvent) -> Element<'a, Message> {
    use kuvpn::EventKind;

    let (dot_color, kind_label) = match event.kind {
        EventKind::Connected => (COLOR_SUCCESS, "Connected"),
        EventKind::Reconnected => (COLOR_WARNING, "Reconnected"),
        EventKind::Disconnected => (COLOR_TEXT_DIM, "Disconnected"),
        EventKind::Cancelled => (COLOR_TEXT_DIM, "Cancelled"),
        EventKind::Error => (Color::from_rgb(0.8, 0.2, 0.2), "Error"),
    };

    let ts = format_timestamp(event.timestamp_unix);
    let duration_str = event
        .duration_secs
        .map(|d| format!(" · {}", format_duration(d)))
        .unwrap_or_default();

    let mut row_content = row![
        container(Space::new().width(8).height(8))
            .width(8)
            .height(8)
            .style(move |_| iced::widget::container::Style {
                background: Some(dot_color.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        // Fixed width so the timestamp column always starts at the same position
        // regardless of which label ("Disconnected" vs "Error" etc.) is shown.
        text(kind_label)
            .size(12)
            .color(dot_color)
            .width(Length::Fixed(88.0)),
        text(format!("{}{}", ts, duration_str))
            .size(11)
            .color(COLOR_TEXT_DIM),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    if let Some(msg) = &event.message {
        row_content = row_content.push(
            text(msg.as_str())
                .size(10)
                .color(COLOR_WARNING)
                .width(Length::Fill),
        );
    }

    container(row_content)
        .padding([6, 10])
        .width(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Color::from_rgb(0.10, 0.10, 0.10).into()),
            border: iced::Border {
                radius: 6.0.into(),
                color: Color::from_rgb(0.20, 0.20, 0.20),
                width: 1.0,
            },
            ..Default::default()
        })
        .into()
}

fn format_timestamp(unix: u64) -> String {
    // Simple formatting without chrono: derive date parts from epoch arithmetic.
    // We need the total seconds since epoch to compute readable date/time.
    let secs = unix;
    let mins = secs / 60;
    let hours_total = mins / 60;
    let days_total = hours_total / 24;

    let sec_part = secs % 60;
    let min_part = mins % 60;
    let hour_part = hours_total % 24;

    // Rough date from days since epoch (ignores leap seconds, good enough for display)
    let (year, month, day) = days_to_ymd(days_total);

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour_part, min_part, sec_part
    )
}

/// Very small Gregorian calendar approximation (no leap-second correction).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Use the proleptic Gregorian algorithm from the civil::date paper
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
