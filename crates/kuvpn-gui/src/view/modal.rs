use crate::app::KuVpnGui;
use crate::types::{
    btn_primary, InputRequest, Message, COLOR_ACCENT, COLOR_SURFACE, COLOR_TEXT, COLOR_TEXT_DIM,
    ICON_EYE_SVG, ICON_LOCK_SVG,
};
use iced::widget::{button, column, container, mouse_area, row, svg, text, text_input};
use iced::{Alignment, Border, Color, Element, Length};

impl KuVpnGui {
    pub fn view_modal<'a>(&self, req: &'a InputRequest) -> Element<'a, Message> {
        // Determine context-aware title
        let title = if req.is_password {
            if req.msg.contains("password to start the VPN tunnel") {
                "System Password"
            } else {
                "KU Account"
            }
        } else {
            "KU Authentication"
        };

        let show_held = self.show_password_held;

        let input_field: Element<'_, Message> = if req.is_password {
            row![
                text_input("Password", &self.current_input)
                    .on_input(Message::InputChanged)
                    .secure(!show_held)
                    .on_submit(Message::SubmitInput)
                    .padding(12)
                    .width(Length::Fill),
                mouse_area(
                    container(
                        svg(svg::Handle::from_memory(ICON_EYE_SVG))
                            .width(18)
                            .height(18)
                            .style(move |_, _| svg::Style {
                                color: Some(if show_held { COLOR_TEXT } else { COLOR_TEXT_DIM }),
                            }),
                    )
                    .padding([12, 12])
                    .style(move |_| container::Style {
                        background: Some(
                            if show_held {
                                Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                            } else {
                                Color::TRANSPARENT
                            }
                            .into(),
                        ),
                        border: Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                )
                .on_press(Message::ShowPasswordHeld(true))
                .on_release(Message::ShowPasswordHeld(false)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            text_input("Response Required", &self.current_input)
                .on_input(Message::InputChanged)
                .on_submit(Message::SubmitInput)
                .padding(12)
                .into()
        };

        let modal_content = container(
            column![
                row![
                    svg(svg::Handle::from_memory(ICON_LOCK_SVG))
                        .width(26)
                        .height(26)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_ACCENT)
                        }),
                    text(title).size(20),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
                text(&req.msg).size(14).color(COLOR_TEXT),
                input_field,
                button(text("VERIFY").size(14).color(Color::WHITE))
                    .width(Length::Fill)
                    .padding([12, 16])
                    .on_press(Message::SubmitInput)
                    .style(btn_primary)
            ]
            .spacing(20)
            .padding(32),
        )
        .width(Length::Fixed(400.0))
        .style(|_| container::Style {
            background: Some(COLOR_SURFACE.into()),
            border: Border {
                radius: 10.0.into(),
                color: COLOR_ACCENT,
                width: 1.0,
            },
            ..Default::default()
        });

        container(modal_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.85).into()),
                ..Default::default()
            })
            .into()
    }
}
