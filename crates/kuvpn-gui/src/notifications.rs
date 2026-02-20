use notify_rust::Notification;

pub fn send_notification(summary: &str, body: &str) {
    let mut notification = Notification::new();
    notification
        .summary(summary)
        .body(body)
        .appname("KUVPN")
        .timeout(5000);

    #[cfg(target_os = "linux")]
    {
        notification.icon("kuvpn");
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, the icon needs to be a path to a file or a resource index.
        // For now, we'll skip the icon or let notify-rust handle it if it can.
    }

    if let Err(e) = notification.show() {
        log::error!("Failed to show notification: {}", e);
    }
}
