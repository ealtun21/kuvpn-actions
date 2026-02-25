/// Platform-specific helpers for bringing the application window to the foreground.
///
/// Iced's `gain_focus` delegates to winit's `focus_window()`, which is correct but
/// gated on the window already being *visible*. When the window was closed (hidden to
/// tray) and we just called `window::open()`, there's a brief window (pun intended)
/// where the native window exists but `focus_window()` might be a no-op.  We call
/// `activate_app()` as a belt-and-suspenders measure alongside Iced's own gain_focus.

/// Attempt to bring the application to the foreground using OS-specific mechanisms.
/// This is called in addition to `iced::window::gain_focus` whenever the app needs
/// the user's attention (MFA prompt, credential input, newly opened window, etc.).
pub fn activate_app() {
    #[cfg(target_os = "macos")]
    activate_macos();

    // Linux (X11/Wayland): Iced's gain_focus → winit → focus_window() is sufficient.
    // Windows: winit's force_window_active() (SendInput alt-trick + SetForegroundWindow)
    //          handles this correctly through Iced's gain_focus path.
}

/// On macOS, use osascript to set our process as frontmost.
///
/// `NSApp.activateIgnoringOtherApps` is already called by winit's focus_window(), but
/// when the app runs as an accessory process (no Dock icon), macOS can be reluctant
/// to switch focus.  The System Events approach via osascript is more reliable and
/// requires no extra Rust dependencies.
#[cfg(target_os = "macos")]
fn activate_macos() {
    let pid = std::process::id();
    // "set frontmost of first process whose unix id is <pid> to true"
    // works for both bundled and unbundled apps.
    let script = format!(
        "tell application \"System Events\" \
         to set frontmost of first process whose unix id is {} to true",
        pid
    );
    // Fire-and-forget — we don't need to wait for this to complete.
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn();
}
