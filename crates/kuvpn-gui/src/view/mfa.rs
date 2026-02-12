use crate::app::KuVpnGui;
use crate::types::{Message, COLOR_WARNING, ICON_PHONE, NERD_FONT};
use iced::widget::{column, container, row, text};
use iced::{Alignment, Border, Color, Element, Length};

impl KuVpnGui {
    pub fn view_mfa_banner(&self) -> Element<'_, Message> {
        if let Some(mfa) = &self.mfa_info {
            container(
                row![
                    text(ICON_PHONE)
                        .font(NERD_FONT)
                        .size(40)
                        .color(COLOR_WARNING),
                    column![
                        text("Approval Required")
                            .size(14)
                            .color(COLOR_WARNING)
                            .font(NERD_FONT),
                        text(mfa).size(22).color(Color::WHITE).font(NERD_FONT),
                    ]
                    .spacing(5)
                ]
                .spacing(20)
                .align_y(Alignment::Center),
            )
            .padding(20)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.80, 0.60, 0.30, 0.05).into()),
                border: Border {
                    color: COLOR_WARNING,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            })
            .into()
        } else {
            container(iced::widget::Space::new().height(0)).into()
        }
    }
}
