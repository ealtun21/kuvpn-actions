use crate::app::KuVpnGui;
use crate::types::{Message, COLOR_WARNING, ICON_PHONE_SVG};
use iced::widget::{column, container, row, svg, text};
use iced::{Alignment, Border, Color, Element, Length};

impl KuVpnGui {
    pub fn view_mfa_banner(&self) -> Element<'_, Message> {
        if let Some(mfa) = &self.mfa_info {
            container(
                row![
                    svg(svg::Handle::from_memory(ICON_PHONE_SVG))
                        .width(32)
                        .height(32)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_WARNING)
                        }),
                    column![
                        text("Approval Required")
                            .size(12)
                            .color(COLOR_WARNING),
                        text(mfa).size(20).color(Color::WHITE),
                    ]
                    .spacing(5)
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding(12)
            .width(Length::Shrink)
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.80, 0.60, 0.30, 0.08).into()),
                border: Border {
                    color: COLOR_WARNING,
                    width: 1.5,
                    radius: 10.0.into(),
                },
                ..Default::default()
            })
            .into()
        } else {
            container(iced::widget::Space::new().height(0)).into()
        }
    }
}
