pub mod actions;
pub mod console;
pub mod header;
pub mod mfa;
pub mod modal;
pub mod settings;
pub mod status;

use crate::app::KuVpnGui;
use crate::types::{Message, COLOR_BG, COLOR_TEXT};
use iced::widget::{column, container, row, scrollable, stack, text};
use iced::{Alignment, Element, Length};

impl KuVpnGui {
    pub fn view(&self, _id: iced::window::Id) -> Element<'_, Message> {
        let header = self.view_header();
        let status_view = self.view_status_circle();
        let mfa_banner = self.view_mfa_banner();
        let action_section = self.view_actions();

        let mut content = column![];

        if self.oc_test_result == Some(false) {
            content = content.push(
                container(
                    row![
                        text(crate::types::ICON_INFO)
                            .font(crate::types::NERD_FONT)
                            .color(crate::types::COLOR_WARNING),
                        text("OpenConnect not found! Please install it or set path in Settings.")
                            .size(14)
                            .color(crate::types::COLOR_WARNING),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(15)
                .style(|_| container::Style {
                    background: Some(crate::types::COLOR_SURFACE.into()),
                    border: iced::Border {
                        color: crate::types::COLOR_WARNING,
                        width: 1.0,
                        radius: 5.0.into(),
                    },
                    ..Default::default()
                }),
            );
        }

        content = content
            .push(header)
            .push(status_view)
            .push(mfa_banner)
            .push(action_section)
            .spacing(30)
            .padding(30)
            .align_x(Alignment::Center)
            .height(Length::Shrink);

        if self.show_advanced {
            content = content.push(self.view_advanced_settings());
        }

        if self.show_console {
            content = content.push(self.view_console());
        }

        let main_container = container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(COLOR_BG.into()),
                text_color: Some(COLOR_TEXT.into()),
                ..Default::default()
            });

        if let Some(req) = &self.pending_request {
            stack![main_container, self.view_modal(req)].into()
        } else {
            main_container.into()
        }
    }
}
