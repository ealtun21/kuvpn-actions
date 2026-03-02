use crate::app::KuVpnGui;
#[cfg_attr(windows, allow(unused_imports))]
use crate::types::COLOR_WARNING;
use crate::types::{
    btn_secondary, btn_segment_selected, btn_segment_unselected, card, custom_scrollbar, Message,
    SegmentPosition, COLOR_SUCCESS, COLOR_TEXT, COLOR_TEXT_DIM, ICON_INFO_SVG, ICON_REFRESH_SVG,
    ICON_TRASH_SVG,
};
use iced::widget::{button, container, row, scrollable, svg, text, text_input, Column};
use iced::{Alignment, Border, Color, Element, Length, Padding, Shadow, Vector};

// ── Tooltip helper ────────────────────────────────────────────────────────────

/// Renders a small (i) icon that shows `tip` as a tooltip on hover.
fn info_tip<'a>(tip: &'a str) -> Element<'a, Message> {
    use iced::widget::tooltip;

    let icon = svg(svg::Handle::from_memory(ICON_INFO_SVG))
        .width(14)
        .height(14)
        .style(|_, _| svg::Style {
            color: Some(COLOR_TEXT_DIM),
        });

    let tip_body = container(text(tip).size(11).color(COLOR_TEXT))
        .padding([7, 10])
        .max_width(260.0)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.09, 0.09, 0.09).into()),
            border: Border {
                radius: 6.0.into(),
                color: Color::from_rgb(0.28, 0.28, 0.28),
                width: 1.0,
            },
            ..Default::default()
        });

    tooltip(icon, tip_body, tooltip::Position::Left)
        .gap(6)
        .into()
}

// ── Main settings view ────────────────────────────────────────────────────────

impl KuVpnGui {
    pub fn view_advanced_settings(&self) -> Element<'_, Message> {
        let is_locked = false;

        let locked_hint = if is_locked {
            container(
                row![
                    svg(svg::Handle::from_memory(ICON_INFO_SVG))
                        .width(14)
                        .height(14)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_TEXT_DIM)
                        }),
                    text("Settings locked during active session.")
                        .size(11)
                        .color(COLOR_TEXT_DIM),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding(5)
        } else {
            container(iced::widget::Space::new().height(0))
        };

        let section_label = |label: &'static str| -> Element<'_, Message> {
            container(
                text(label)
                    .size(10)
                    .color(Color::from_rgb(0.45, 0.45, 0.45)),
            )
            .padding([8, 0])
            .into()
        };

        let divider = || -> Element<'_, Message> {
            container(iced::widget::Space::new().height(0))
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.20, 0.20, 0.20).into()),
                    ..Default::default()
                })
                .into()
        };

        // Inline notification shown when the Test button resolved a different path.
        let oc_path_notif: Element<'_, Message> = if let Some(msg) = &self.oc_path_notification {
            container(
                row![
                    svg(svg::Handle::from_memory(ICON_INFO_SVG))
                        .width(13)
                        .height(13)
                        .style(|_, _| svg::Style {
                            color: Some(COLOR_SUCCESS)
                        }),
                    text(msg.as_str()).size(10).color(COLOR_SUCCESS),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([6, 110])
            .style(|_| container::Style {
                background: Some(Color::from_rgba(0.42, 0.55, 0.35, 0.07).into()),
                border: Border {
                    radius: 6.0.into(),
                    color: Color::from_rgba(0.42, 0.55, 0.35, 0.25),
                    width: 1.0,
                },
                ..Default::default()
            })
            .into()
        } else {
            iced::widget::Space::new().height(0).into()
        };

        // ── Header row ────────────────────────────────────────────────────────
        let header = row![
            text("Settings")
                .size(14)
                .color(COLOR_TEXT)
                .width(Length::Fill),
            self.view_segmented_control(
                &["Basic", "Advanced"],
                &[0.0, 1.0],
                if self.settings.advanced_mode {
                    1.0
                } else {
                    0.0
                },
                false,
                |val| Message::AdvancedModeToggled(val > 0.5)
            ),
            locked_hint,
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        // ── Build settings column dynamically ─────────────────────────────────
        let adv = self.settings.advanced_mode;
        let mut col: Column<'_, Message> = Column::new().spacing(12);

        col = col.push(header);

        // BROWSER section
        col = col.push(section_label("BROWSER"));
        col = col.push(self.view_unified_field(
            "KU Email:",
            "username@ku.edu.tr",
            &self.settings.email,
            "Your KU email address (e.g. jdoe24@ku.edu.tr). Saved locally so you don't re-enter after wiping session. Used in Full Auto and Visual Auto modes to pre-fill the login form.",
            is_locked,
            Message::EmailChanged,
        ));
        col = col.push(self.view_unified_control(
            "Login Mode:",
            self.view_segmented_control(
                &["Full Auto", "Visual Auto", "Manual"],
                &[0.0, 1.0, 2.0],
                self.settings.login_mode_val,
                is_locked,
                Message::LoginModeChanged,
            ),
            "Full Auto: fully hands-free login via browser automation — no interaction needed.\n\nVisual Auto: same automation but shows the browser window, useful for debugging.\n\nManual: the browser opens and you complete the login yourself.",
        ));

        // NETWORK section (advanced only)
        if adv {
            col = col.push(divider());
            col = col.push(section_label("NETWORK"));
            col = col.push(self.view_unified_field(
                "Gateway URL:",
                "https://vpn.example.com",
                &self.settings.url,
                "The HTTPS address of the KU VPN gateway server. Leave as the default (https://vpn.ku.edu.tr) unless IT Support instructs you to use a different server.",
                is_locked,
                Message::UrlChanged,
            ));
            col = col.push(self.view_unified_field(
                "DSID Domain:",
                "vpn.example.com",
                &self.settings.domain,
                "Hostname used to extract the DSID session cookie after login. Must match the domain of your Gateway URL. Only change this if you changed the Gateway URL.",
                is_locked,
                Message::DomainChanged,
            ));

            // OC Path row (has a custom layout with Test button)
            {
                #[cfg(windows)]
                let oc_placeholder = r"openconnect\openconnect.exe";
                #[cfg(not(windows))]
                let oc_placeholder = "openconnect";

                #[cfg(windows)]
                let oc_tip = "Path to openconnect.exe. Defaults to the bundled binary in the app directory. Click Test to verify or auto-detect the path.";
                #[cfg(not(windows))]
                let oc_tip = "Path to the openconnect binary. Leave blank to use 'openconnect' from your system PATH. Click Test to auto-detect the installed binary.";

                let oc_row = row![
                    text("OC Path:").size(11).width(Length::Fixed(100.0)),
                    text_input(oc_placeholder, &self.settings.openconnect_path)
                        .on_input(if is_locked {
                            |_| Message::Tick
                        } else {
                            Message::OpenConnectPathChanged
                        })
                        .padding(10)
                        .width(Length::Fill)
                        .style(move |_theme, status| {
                            let mut style = text_input::default(_theme, status);
                            style.background =
                                iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.08));
                            style.border = Border {
                                color: match status {
                                    text_input::Status::Active => Color::from_rgb(0.20, 0.20, 0.20),
                                    text_input::Status::Focused { is_hovered } => {
                                        if is_hovered {
                                            Color::from_rgb(0.35, 0.35, 0.35)
                                        } else {
                                            Color::from_rgb(0.30, 0.30, 0.30)
                                        }
                                    }
                                    text_input::Status::Hovered => {
                                        Color::from_rgb(0.25, 0.25, 0.25)
                                    }
                                    text_input::Status::Disabled => {
                                        Color::from_rgb(0.15, 0.15, 0.15)
                                    }
                                },
                                width: 1.0,
                                radius: 6.0.into(),
                            };
                            style
                        }),
                    button(
                        text(if self.oc_test_result == Some(true) {
                            "✓"
                        } else if self.oc_test_result == Some(false) {
                            "✗"
                        } else {
                            "Test"
                        })
                        .size(11),
                    )
                    .padding([8, 12])
                    .on_press(if is_locked {
                        Message::Tick
                    } else {
                        Message::TestOpenConnect
                    })
                    .style(if self.oc_test_result == Some(true) {
                        button::success
                    } else if self.oc_test_result == Some(false) {
                        button::danger
                    } else {
                        button::secondary
                    }),
                    info_tip(oc_tip),
                ]
                .spacing(10)
                .align_y(Alignment::Center);

                col = col.push(oc_row);
                col = col.push(oc_path_notif);
            }
        }

        // SYSTEM section
        col = col.push(divider());
        col = col.push(section_label("SYSTEM"));

        // Log Level and Elevation are advanced-only
        if adv {
            col = col.push(self.view_unified_control(
                "Log Level:",
                self.view_segmented_control(
                    &["Off", "Error", "Warn", "Info", "Debug", "Trace"],
                    &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
                    self.settings.log_level_val,
                    false,
                    Message::LogLevelSliderChanged,
                ),
                "Controls what gets recorded in the Session Log tab. Info is recommended for everyday use. Debug and Trace add detailed technical output for troubleshooting. Off disables all logging.",
            ));

            #[cfg(not(windows))]
            {
                if self.available_escalation_tools.is_empty() {
                    col = col.push(self.view_unified_control(
                        "Elevation:",
                        container(
                            row![
                                svg(svg::Handle::from_memory(ICON_INFO_SVG))
                                    .width(13)
                                    .height(13)
                                    .style(|_, _| svg::Style {
                                        color: Some(COLOR_WARNING)
                                    }),
                                text("No privilege tool found! Install sudo or pkexec.")
                                    .size(11)
                                    .color(COLOR_WARNING),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center),
                        )
                        .into(),
                        "The privilege escalation tool used to launch OpenConnect as root (required to create the VPN tunnel). Install sudo or pkexec to proceed.",
                    ));
                } else {
                    col = col.push(self.view_unified_control(
                        "Elevation:",
                        self.view_segmented_control_str(
                            &self.available_escalation_tools,
                            &self.settings.escalation_tool,
                            is_locked,
                            Message::EscalationToolChanged,
                        ),
                        "The privilege escalation tool used to launch OpenConnect as root (required to create the VPN network interface). Only tools installed on your system are listed.",
                    ));
                }
            }
        }

        col = col.push(self.view_unified_control(
            "Close to Tray:",
            self.view_segmented_control(
                &["Yes", "No"],
                &[1.0, 0.0],
                if self.settings.close_to_tray { 1.0 } else { 0.0 },
                false,
                |val| Message::CloseToTrayToggled(val > 0.5),
            ),
            "When Yes, clicking the window × button minimizes to the system tray instead of quitting — the VPN stays connected. When No, clicking × exits the app and disconnects.",
        ));
        col = col.push(self.view_unified_control(
            "Auto-hide:",
            self.view_segmented_control(
                &["Yes", "No"],
                &[1.0, 0.0],
                if self.settings.auto_hide_after_prompt { 1.0 } else { 0.0 },
                false,
                |val| Message::AutoHideAfterPromptToggled(val > 0.5),
            ),
            "When Yes, the window automatically hides after a login prompt resolves, if the window was originally shown automatically from the tray to display that prompt.",
        ));
        col = col.push(self.view_unified_control(
            "Window Style:",
            self.view_segmented_control(
                &["System", "Custom"],
                &[0.0, 1.0],
                if self.settings.use_client_decorations { 1.0 } else { 0.0 },
                false,
                |val| Message::ClientDecorationsToggled(val > 0.5),
            ),
            "System: uses your OS's native window borders and titlebar.\n\nCustom: frameless window with a built-in dark titlebar — matches the app theme better.",
        ));

        // ACTIONS section
        col = col.push(divider());
        col = col.push(section_label("ACTIONS"));
        {
            let fade = self.notif_fade;

            let wipe_btn: Element<'_, Message> = if let Some(success) = self.session_wipe_result {
                let (label, r, g, b) = if success {
                    ("✓  WIPED", 0.42f32, 0.55f32, 0.35f32)
                } else {
                    ("✗  FAILED", 0.8f32, 0.2f32, 0.2f32)
                };
                button(
                    text(label)
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, fade)),
                )
                .padding([10, 14])
                .on_press(Message::ClearSessionPressed)
                .style(move |_, _| button::Style {
                    background: Some(Color::from_rgba(r, g, b, 0.55 * fade).into()),
                    border: Border {
                        radius: 10.0.into(),
                        color: Color::from_rgba(r, g, b, 0.75 * fade),
                        width: 1.5,
                    },
                    shadow: Shadow {
                        color: Color::from_rgba(r, g, b, 0.45 * fade),
                        offset: Vector::new(0.0, 0.0),
                        blur_radius: 12.0,
                    },
                    ..Default::default()
                })
                .into()
            } else {
                button(
                    row![
                        svg(svg::Handle::from_memory(ICON_TRASH_SVG))
                            .width(13)
                            .height(13)
                            .style(|_, _| svg::Style {
                                color: Some(COLOR_TEXT)
                            }),
                        text("WIPE SESSION").size(11).color(COLOR_TEXT),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                )
                .padding([10, 14])
                .on_press(Message::ClearSessionPressed)
                .style(btn_secondary)
                .into()
            };

            let reset_btn: Element<'_, Message> = if self.reset_notification {
                button(
                    text("✓  RESTORED")
                        .size(11)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, fade)),
                )
                .padding([10, 14])
                .on_press(if is_locked {
                    Message::Tick
                } else {
                    Message::ResetSettings
                })
                .style(move |_, _| button::Style {
                    background: Some(Color::from_rgba(0.42, 0.55, 0.35, 0.55 * fade).into()),
                    border: Border {
                        radius: 10.0.into(),
                        color: Color::from_rgba(0.42, 0.55, 0.35, 0.75 * fade),
                        width: 1.5,
                    },
                    shadow: Shadow {
                        color: Color::from_rgba(0.42, 0.55, 0.35, 0.45 * fade),
                        offset: Vector::new(0.0, 0.0),
                        blur_radius: 12.0,
                    },
                    ..Default::default()
                })
                .into()
            } else {
                button(
                    row![
                        svg(svg::Handle::from_memory(ICON_REFRESH_SVG))
                            .width(13)
                            .height(13)
                            .style(|_, _| svg::Style {
                                color: Some(COLOR_TEXT)
                            }),
                        text("RESET DEFAULTS").size(11).color(COLOR_TEXT),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                )
                .padding([10, 14])
                .on_press(if is_locked {
                    Message::Tick
                } else {
                    Message::ResetSettings
                })
                .style(btn_secondary)
                .into()
            };

            col = col.push(row([wipe_btn, reset_btn]).spacing(10));
        }

        container(
            scrollable(container(col).padding(Padding {
                top: 0.0,
                right: 24.0,
                bottom: 0.0,
                left: 0.0,
            }))
            .height(Length::Fill)
            .style(custom_scrollbar),
        )
        .padding(24)
        .width(Length::Fill)
        .style(card)
        .into()
    }

    // ── Field helpers ─────────────────────────────────────────────────────────

    fn view_unified_field<'a>(
        &self,
        label: &'a str,
        placeholder: &'a str,
        value: &'a str,
        tooltip_text: &'a str,
        locked: bool,
        on_change: fn(String) -> Message,
    ) -> Element<'a, Message> {
        row![
            text(label).size(11).width(Length::Fixed(100.0)),
            text_input(placeholder, value)
                .on_input(if locked { |_| Message::Tick } else { on_change })
                .padding(10)
                .width(Length::Fill)
                .style(move |_theme, status| {
                    let mut style = text_input::default(_theme, status);
                    style.background = iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.08));
                    style.border = Border {
                        color: match status {
                            text_input::Status::Active => Color::from_rgb(0.20, 0.20, 0.20),
                            text_input::Status::Focused { is_hovered } => {
                                if is_hovered {
                                    Color::from_rgb(0.35, 0.35, 0.35)
                                } else {
                                    Color::from_rgb(0.30, 0.30, 0.30)
                                }
                            }
                            text_input::Status::Hovered => Color::from_rgb(0.25, 0.25, 0.25),
                            text_input::Status::Disabled => Color::from_rgb(0.15, 0.15, 0.15),
                        },
                        width: 1.0,
                        radius: 6.0.into(),
                    };
                    style
                }),
            info_tip(tooltip_text),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_unified_control<'a>(
        &self,
        label: &'a str,
        control: Element<'a, Message>,
        tooltip_text: &'a str,
    ) -> Element<'a, Message> {
        row![
            text(label).size(11).width(Length::Fixed(100.0)),
            control,
            info_tip(tooltip_text),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }

    // ── Segmented controls ────────────────────────────────────────────────────

    fn view_segmented_control<'a>(
        &self,
        options: &'a [&'static str],
        values: &'a [f32],
        current_value: f32,
        locked: bool,
        on_change: fn(f32) -> Message,
    ) -> Element<'a, Message> {
        let buttons: Vec<Element<'a, Message>> = options
            .iter()
            .zip(values.iter())
            .enumerate()
            .map(|(idx, (label, &value))| {
                let is_selected = (current_value - value).abs() < 0.1;

                let position = if options.len() == 1 {
                    SegmentPosition::Single
                } else if idx == 0 {
                    SegmentPosition::Left
                } else if idx == options.len() - 1 {
                    SegmentPosition::Right
                } else {
                    SegmentPosition::Middle
                };

                button(text(*label).size(11))
                    .padding([9, 12])
                    .height(Length::Fixed(34.0))
                    .on_press(if locked {
                        Message::Tick
                    } else {
                        on_change(value)
                    })
                    .style(move |theme, status| {
                        if is_selected {
                            btn_segment_selected(theme, status, position)
                        } else {
                            btn_segment_unselected(theme, status, position)
                        }
                    })
                    .into()
            })
            .collect();

        row(buttons).spacing(-1.0).into()
    }

    #[cfg(not(windows))]
    fn view_segmented_control_str<'a>(
        &self,
        options: &'a [&'static str],
        current_value: &str,
        locked: bool,
        on_change: fn(String) -> Message,
    ) -> Element<'a, Message> {
        let buttons: Vec<Element<'a, Message>> = options
            .iter()
            .enumerate()
            .map(|(idx, &label)| {
                let is_selected = current_value == label;

                let position = if options.len() == 1 {
                    SegmentPosition::Single
                } else if idx == 0 {
                    SegmentPosition::Left
                } else if idx == options.len() - 1 {
                    SegmentPosition::Right
                } else {
                    SegmentPosition::Middle
                };

                button(text(label).size(11))
                    .padding([9, 12])
                    .height(Length::Fixed(34.0))
                    .on_press(if locked {
                        Message::Tick
                    } else {
                        on_change(label.to_string())
                    })
                    .style(move |theme, status| {
                        if is_selected {
                            btn_segment_selected(theme, status, position)
                        } else {
                            btn_segment_unselected(theme, status, position)
                        }
                    })
                    .into()
            })
            .collect();

        row(buttons).spacing(-1.0).into()
    }
}
