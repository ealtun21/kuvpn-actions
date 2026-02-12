use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Border, Color, Element, Length};
use crate::app::KuVpnGui;
use crate::types::{InputRequest, Message, COLOR_ACCENT, COLOR_SURFACE, COLOR_TEXT, ICON_LOCK, NERD_FONT};

impl KuVpnGui {
    pub fn view_modal<'a>(&self, req: &'a InputRequest) -> Element<'a, Message> {
        let modal_content = container(
            column![
                row![
                    text(ICON_LOCK).font(NERD_FONT).size(30).color(COLOR_ACCENT),
                    text("Campus Gateway").size(24).font(NERD_FONT),
                ].spacing(15).align_y(Alignment::Center),
                
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
                        .font(NERD_FONT)
                )
                .padding(12)
                .on_press(Message::SubmitInput)
                .style(button::primary)
            ]
            .spacing(25)
            .padding(40)
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
