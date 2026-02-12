mod app;
mod config;
mod logger;
mod provider;
mod tray;
mod types;
mod view;

use crate::app::KuVpnGui;
use crate::tray::init_tray;
use crate::types::{Message, NERD_FONT, NERD_FONT_BYTES};
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

                ..Default::default()
            });

            gui.window_id = Some(id);

            (
                gui,
                Task::batch(vec![
                    iced::font::load(NERD_FONT_BYTES).map(|res| {
                        match res {
                            Ok(_) => log::info!("Font loaded successfully"),
                            Err(e) => log::error!("Failed to load font: {:?}", e),
                        }
                        Message::GtkTick
                    }),
                    task.map(Message::WindowOpened),
                    Task::done(Message::TestOpenConnect),
                ]),
            )
        },
        KuVpnGui::update,
        KuVpnGui::view,
    )
    .title(get_title)
    .default_font(NERD_FONT)
    .subscription(get_subscription)
    .theme(get_theme)
    .run()
}
