#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod logger;
mod provider;
mod tray;
mod types;
mod view;

use crate::app::KuVpnGui;
use crate::tray::init_tray;
use crate::types::Message;
use iced::Task;
use std::sync::{Arc, Mutex};

fn get_title(_: &KuVpnGui, _: iced::window::Id) -> String {
    "KUVPN".to_string()
}

fn get_theme(gui: &KuVpnGui, id: iced::window::Id) -> iced::Theme {
    gui.theme(id)
}

fn get_subscription(gui: &KuVpnGui) -> iced::Subscription<Message> {
    gui.subscription()
}

pub fn main() -> iced::Result {
    // Ensure only one instance is running
    if let Err(e) = kuvpn::utils::ensure_single_instance() {
        eprintln!("{}", e);
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        // tray-icon on Linux requires GTK to be initialized first
        let _ = gtk::init();
    }

    let components = Arc::new(Mutex::new(Some(init_tray())));

    iced::daemon(
        move || {
            let mut gui = KuVpnGui::default();

            if let Ok(mut guard) = components.lock() {
                if let Some(c) = guard.take() {
                    gui.tray_icon = Some(c.tray);
                    gui.show_item = Some(c.show_item);
                    gui.connect_item = Some(c.connect_item);
                    gui.disconnect_item = Some(c.disconnect_item);
                }
            }

            let (id, task) = iced::window::open(iced::window::Settings {
                exit_on_close_request: false,
                size: iced::Size::new(480.0, 820.0),
                min_size: Some(iced::Size::new(400.0, 600.0)),
                ..Default::default()
            });

            gui.window_id = Some(id);

            (
                gui,
                Task::batch(vec![
                    task.map(Message::WindowOpened),
                    Task::done(Message::TestOpenConnect),
                ]),
            )
        },
        KuVpnGui::update,
        KuVpnGui::view,
    )
    .title(get_title)
    .subscription(get_subscription)
    .theme(get_theme)
    .run()
}
