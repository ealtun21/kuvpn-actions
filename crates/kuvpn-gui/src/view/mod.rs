pub mod actions;
pub mod console;
pub mod header;
pub mod mfa;
pub mod modal;
pub mod settings;
pub mod status;

use crate::app::KuVpnGui;
use crate::types::{Message, Tab, COLOR_ACCENT, COLOR_BG, COLOR_SURFACE, COLOR_TEXT, COLOR_TEXT_DIM};
use iced::widget::{button, column, container, mouse_area, row, stack, svg, text};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector};

impl KuVpnGui {
    fn view_title_bar(&self) -> Element<'_, Message> {
        let title_bar_content = row![
            svg(svg::Handle::from_memory(crate::types::KU_LOGO_BYTES))
                .width(20)
                .height(20)
                .style(|_, _| svg::Style {
                    color: Some(iced::Color::WHITE)
                }),
            text("KUVPN")
                .size(14)
                .color(iced::Color::WHITE)
                .width(Length::Fill),
            button(text("−").size(18).color(COLOR_TEXT))
                .padding([0, 12])
                .on_press(Message::MinimizeWindow)
                .style(|_theme, status| button::Style {
                    background: Some(Color::TRANSPARENT.into()),
                    text_color: COLOR_TEXT,
                    border: Border::default(),
                    shadow: Shadow::default(),
                    ..match status {
                        button::Status::Hovered => button::Style {
                            background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.1).into()),
                            ..Default::default()
                        },
                        _ => Default::default(),
                    }
                }),
            button(text("✕").size(14).color(COLOR_TEXT))
                .padding([0, 12])
                .on_press(Message::ToggleVisibility {
                    from_close_request: true,
                })
                .style(|_theme, status| button::Style {
                    background: Some(Color::TRANSPARENT.into()),
                    text_color: COLOR_TEXT,
                    border: Border::default(),
                    shadow: Shadow::default(),
                    ..match status {
                        button::Status::Hovered => button::Style {
                            background: Some(Color::from_rgba(0.8, 0.2, 0.2, 0.8).into()),
                            text_color: Color::WHITE,
                            ..Default::default()
                        },
                        _ => Default::default(),
                    }
                }),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([8, 12]);

        mouse_area(
            container(title_bar_content)
                .width(Length::Fill)
                .style(|_| container::Style {
                    background: Some(COLOR_SURFACE.into()),
                    border: Border {
                        radius: iced::border::Radius {
                            top_left: 12.0,
                            top_right: 12.0,
                            bottom_left: 0.0,
                            bottom_right: 0.0,
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                })
        )
        .on_press(Message::DragWindow)
        .into()
    }

    pub fn view(&self, _id: iced::window::Id) -> Element<'_, Message> {
        let use_csd = self.settings.use_client_decorations;

        // OpenConnect warning banner (always visible if needed)
        let oc_warning: Element<'_, Message> = if self.oc_test_result == Some(false) {
            container(
                row![
                    svg(svg::Handle::from_memory(crate::types::ICON_INFO_SVG))
                        .width(14)
                        .height(14)
                        .style(|_, _| svg::Style {
                            color: Some(crate::types::COLOR_WARNING)
                        }),
                    text("OpenConnect not found! Please install it or set path in Settings.")
                        .size(12)
                        .color(crate::types::COLOR_WARNING),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding(10)
            .style(|_| container::Style {
                background: Some(crate::types::COLOR_SURFACE.into()),
                border: Border {
                    color: crate::types::COLOR_WARNING,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .into()
        } else {
            container(iced::widget::Space::new().height(0)).into()
        };

        // Tab bar
        let tab_bar = self.view_tab_bar();

        // Tab content
        let tab_content = match self.current_tab {
            Tab::Connection => self.view_connection_tab(),
            Tab::Settings => self.view_settings_tab(),
            Tab::Console => self.view_console_tab(),
        };

        let content = container(
            column![oc_warning, tab_bar, tab_content]
                .spacing(18)
                .width(Length::Fill)
        )
        .padding(20)
        .width(Length::Fill)
        .max_width(680.0)
        .center_x(Length::Fill);

        // Build content with or without custom title bar
        let window_content = if use_csd {
            let title_bar = self.view_title_bar();
            container(
                column![title_bar, content]
                    .width(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(COLOR_BG.into()),
                text_color: Some(COLOR_TEXT.into()),
                border: Border {
                    color: Color::from_rgb(0.20, 0.20, 0.20),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.8),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 32.0,
                },
                ..Default::default()
            })
        } else {
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style {
                    background: Some(COLOR_BG.into()),
                    text_color: Some(COLOR_TEXT.into()),
                    ..Default::default()
                })
        };

        let main_container = if use_csd {
            container(window_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(8)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
        } else {
            container(window_content)
                .width(Length::Fill)
                .height(Length::Fill)
        };

        if let Some(req) = &self.pending_request {
            stack![main_container, self.view_modal(req)].into()
        } else {
            main_container.into()
        }
    }

    fn view_tab_bar(&self) -> Element<'_, Message> {
        // Connection tab button
        let conn_active = self.current_tab == Tab::Connection;
        let conn_btn = button(
            row![
                svg(svg::Handle::from_memory(crate::types::ICON_SHIELD_SVG))
                    .width(16)
                    .height(16)
                    .style(move |_theme: &iced::Theme, _status| svg::Style {
                        color: Some(if conn_active {
                            iced::Color::WHITE
                        } else {
                            COLOR_TEXT_DIM
                        })
                    }),
                text("Connection").size(13),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([10, 18])
        .width(Length::Fixed(130.0))
        .on_press(Message::TabChanged(Tab::Connection))
        .style(move |_theme, status| {
            if conn_active {
                button::Style {
                    background: Some(COLOR_ACCENT.into()),
                    text_color: iced::Color::WHITE,
                    border: Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                let base = button::Style {
                    background: Some(iced::Color::TRANSPARENT.into()),
                    text_color: COLOR_TEXT_DIM,
                    border: Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                match status {
                    button::Status::Hovered => button::Style {
                        background: Some(crate::types::COLOR_SURFACE.into()),
                        text_color: COLOR_TEXT,
                        ..base
                    },
                    _ => base,
                }
            }
        });

        // Settings tab button
        let settings_active = self.current_tab == Tab::Settings;
        let settings_btn = button(
            row![
                svg(svg::Handle::from_memory(crate::types::ICON_SETTINGS_SVG))
                    .width(16)
                    .height(16)
                    .style(move |_theme: &iced::Theme, _status| svg::Style {
                        color: Some(if settings_active {
                            iced::Color::WHITE
                        } else {
                            COLOR_TEXT_DIM
                        })
                    }),
                text("Settings").size(13),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([10, 18])
        .width(Length::Fixed(110.0))
        .on_press(Message::TabChanged(Tab::Settings))
        .style(move |_theme, status| {
            if settings_active {
                button::Style {
                    background: Some(COLOR_ACCENT.into()),
                    text_color: iced::Color::WHITE,
                    border: Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                let base = button::Style {
                    background: Some(iced::Color::TRANSPARENT.into()),
                    text_color: COLOR_TEXT_DIM,
                    border: Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                match status {
                    button::Status::Hovered => button::Style {
                        background: Some(crate::types::COLOR_SURFACE.into()),
                        text_color: COLOR_TEXT,
                        ..base
                    },
                    _ => base,
                }
            }
        });

        // Console tab button
        let console_active = self.current_tab == Tab::Console;
        let console_btn = button(
            row![
                svg(svg::Handle::from_memory(crate::types::ICON_TERMINAL_SVG))
                    .width(16)
                    .height(16)
                    .style(move |_theme: &iced::Theme, _status| svg::Style {
                        color: Some(if console_active {
                            iced::Color::WHITE
                        } else {
                            COLOR_TEXT_DIM
                        })
                    }),
                text("Console").size(13),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([10, 18])
        .width(Length::Fixed(105.0))
        .on_press(Message::TabChanged(Tab::Console))
        .style(move |_theme, status| {
            if console_active {
                button::Style {
                    background: Some(COLOR_ACCENT.into()),
                    text_color: iced::Color::WHITE,
                    border: Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                let base = button::Style {
                    background: Some(iced::Color::TRANSPARENT.into()),
                    text_color: COLOR_TEXT_DIM,
                    border: Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                match status {
                    button::Status::Hovered => button::Style {
                        background: Some(crate::types::COLOR_SURFACE.into()),
                        text_color: COLOR_TEXT,
                        ..base
                    },
                    _ => base,
                }
            }
        });

        row![conn_btn, settings_btn, console_btn]
            .spacing(8)
            .into()
    }

    fn view_connection_tab(&self) -> Element<'_, Message> {
        let status_view = self.view_status_circle();
        let mfa_banner = self.view_mfa_banner();
        let action_section = self.view_actions();

        // Automation warning banner
        let automation_warning_banner: Element<'_, Message> =
            if let Some(warning) = &self.automation_warning {
                container(
                    column![
                        row![
                            svg(svg::Handle::from_memory(crate::types::ICON_INFO_SVG))
                                .width(18)
                                .height(18)
                                .style(|_theme: &iced::Theme, _status| svg::Style {
                                    color: Some(crate::types::COLOR_WARNING)
                                }),
                            text("Automation Issue")
                                .size(13)
                                .color(crate::types::COLOR_WARNING),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        text(warning).size(11).color(crate::types::COLOR_TEXT),
                    ]
                    .spacing(6),
                )
                .width(Length::Fill)
                .max_width(480.0)
                .padding(14)
                .style(|_| container::Style {
                    background: Some(iced::Color::from_rgba(0.80, 0.60, 0.30, 0.08).into()),
                    border: Border {
                        color: crate::types::COLOR_WARNING,
                        width: 1.5,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
                .into()
            } else {
                container(iced::widget::Space::new().height(0)).into()
            };

        // Main connection card
        container(
            column![
                status_view,
                mfa_banner,
                automation_warning_banner,
                action_section
            ]
            .spacing(24)
            .align_x(Alignment::Center)
            .padding([30, 20])
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(crate::types::card)
        .into()
    }

    fn view_settings_tab(&self) -> Element<'_, Message> {
        self.view_advanced_settings()
    }

    fn view_console_tab(&self) -> Element<'_, Message> {
        self.view_console()
    }
}
