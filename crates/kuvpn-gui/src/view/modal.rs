use crate::app::KuVpnGui;
use crate::types::{
    btn_primary, InputRequest, Message, COLOR_ACCENT, COLOR_SURFACE, COLOR_TEXT, ICON_LOCK_SVG,
};
use iced::widget::{button, column, container, row, svg, text, text_input};
use iced::{Alignment, Border, Color, Element, Length};

impl KuVpnGui {
    pub fn view_modal<'a>(&self, req: &'a InputRequest) -> Element<'a, Message> {
        let modal_content = container(
            column![
                row![
                    svg(svg::Handle::from_memory(ICON_LOCK_SVG))
                        .width(30)
                        .height(30)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_ACCENT)
                        }),
                    text("Campus Gateway").size(24),
                ]
                .spacing(15)
                .align_y(Alignment::Center),
                text(&req.msg).size(16).color(COLOR_TEXT),
                if req.is_password {
                    text_input("Credentials", &self.current_input)
                        .on_input(Message::InputChanged)
                        .secure(true)
                        .on_submit(Message::SubmitInput)
                        .padding(15)
                } else {
                    text_input("Response Required", &self.current_input)
                        .on_input(Message::InputChanged)
                        .on_submit(Message::SubmitInput)
                        .padding(15)
                },
                button(
                    text("VERIFY")
                        .width(Length::Fill)
                        .align_x(Alignment::Center)
                )
                .padding(12)
                .on_press(Message::SubmitInput)
                .style(btn_primary)
            ]
            .spacing(25)
            .padding(40),
        )
        .width(Length::Fixed(450.0))
        .style(|_| container::Style {
            background: Some(COLOR_SURFACE.into()),
            border: Border {
                radius: 12.0.into(),
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
