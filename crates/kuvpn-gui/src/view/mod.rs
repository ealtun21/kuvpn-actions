pub mod header;
pub mod status;
pub mod mfa;
pub mod actions;
pub mod settings;
pub mod console;
pub mod modal;

use iced::widget::{column, container, scrollable, stack};
use iced::{Alignment, Element, Length};
use crate::app::KuVpnGui;
use crate::types::{Message, COLOR_BG, COLOR_TEXT};

impl KuVpnGui {
    pub fn view(&self, _id: iced::window::Id) -> Element<'_, Message> {
        let header = self.view_header();
        let status_view = self.view_status_circle();
        let mfa_banner = self.view_mfa_banner();
        let action_section = self.view_actions();
        
        let mut content = column![
            header,
            status_view,
            mfa_banner,
            action_section,
        ]
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
            stack![
                main_container,
                self.view_modal(req)
            ].into()
        } else {
            main_container.into()
        }
    }
}
