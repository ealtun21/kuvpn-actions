use tray_icon::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};

pub struct TrayComponents {
    pub tray: TrayIcon,
    pub show_item: MenuItem,
    pub connect_item: MenuItem,
    pub disconnect_item: MenuItem,
}

pub fn init_tray() -> TrayComponents {
    let show_item = MenuItem::with_id("show", "Toggle Visibility", true, None);
    let connect_item = MenuItem::with_id("connect", "Connect", true, None);
    let disconnect_item = MenuItem::with_id("disconnect", "Disconnect", true, None);
    let quit_item = MenuItem::with_id("quit", "Quit", true, None);

    let tray_menu = Menu::with_items(&[
        &show_item,
        &connect_item,
        &disconnect_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ])
    .expect("Failed to create tray menu");

    let icon = {
        let mut rgba = vec![0u8; 32 * 32 * 4];
        for i in 0..32 * 32 {
            rgba[i * 4] = 128; // R
            rgba[i * 4 + 1] = 0; // G
            rgba[i * 4 + 2] = 32; // B
            rgba[i * 4 + 3] = 255; // A
        }
        tray_icon::Icon::from_rgba(rgba, 32, 32).expect("Failed to create icon")
    };

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("KUVPN")
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon");

    TrayComponents {
        tray,
        show_item,
        connect_item,
        disconnect_item,
    }
}
