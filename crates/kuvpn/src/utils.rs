use console::{Style, Term};
use dialoguer::{Input, Password};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Trait for providing credentials and user input.
///
/// `request_text` / `request_password` return `None` when the prompt was
/// dismissed externally (e.g. the page changed while the user was typing).
pub trait CredentialsProvider: Send + Sync {
    fn request_text(&self, msg: &str) -> Option<String>;
    fn request_password(&self, msg: &str) -> Option<String>;
    fn on_mfa_push(&self, _code: &str) {}
    fn on_mfa_complete(&self) {}

    /// Install a guard that is polled while a prompt is visible.
    /// Return `false` → page changed → prompt is dismissed and `request_*` returns `None`.
    fn set_page_guard(&self, _guard: Box<dyn Fn() -> bool + Send + Sync>) {}

    /// Remove any active page guard.
    fn clear_page_guard(&self) {}
}

/// A implementation of `CredentialsProvider` for terminal input.
pub struct TerminalCredentialsProvider;

impl CredentialsProvider for TerminalCredentialsProvider {
    fn request_text(&self, msg: &str) -> Option<String> {
        // Clear the current line in case a spinner is active on another thread
        let term = Term::stderr();
        let _ = term.clear_line();
        Some(
            Input::new()
                .with_prompt(msg.trim_end_matches(": ").trim_end_matches(':'))
                .interact_text()
                .unwrap_or_default(),
        )
    }

    fn request_password(&self, msg: &str) -> Option<String> {
        // Clear the current line in case a spinner is active on another thread
        let term = Term::stderr();
        let _ = term.clear_line();
        Some(
            Password::new()
                .with_prompt(msg.trim_end_matches(": ").trim_end_matches(':'))
                .interact()
                .unwrap_or_default(),
        )
    }

    fn on_mfa_push(&self, code: &str) {
        let bold = Style::new().bold();
        let cyan = Style::new().cyan().bold();
        eprintln!();
        eprintln!(
            "{} Enter {} in Authenticator",
            bold.apply_to(">>"),
            cyan.apply_to(code),
        );
        eprintln!();
    }

    fn on_mfa_complete(&self) {
        let green = Style::new().green();
        eprintln!("  {} MFA approved", green.apply_to("✓"));
    }
}

use fd_lock::{RwLock, RwLockWriteGuard};
use once_cell::sync::Lazy;
use std::fs::File;
use std::sync::Mutex;

// Stores the write guard to keep the lock held for the process lifetime.
static INSTANCE_LOCK: Lazy<Mutex<Option<RwLockWriteGuard<'static, File>>>> =
    Lazy::new(|| Mutex::new(None));

/// Ensures that only one instance of the application is running.
/// Returns an error if another instance is already active.
pub fn ensure_single_instance() -> Result<(), Box<dyn Error>> {
    let mut lock_path = get_user_data_dir()?;
    lock_path.pop(); // Go to parent of 'profile' (~/.local/share/kuvpn/)
    std::fs::create_dir_all(&lock_path)?;
    lock_path.push("kuvpn.lock");

    let file = File::create(&lock_path)?;
    // Leak the RwLock to get a 'static reference so the guard can outlive this function.
    let lock_ref: &'static mut RwLock<File> = Box::leak(Box::new(RwLock::new(file)));

    match lock_ref.try_write() {
        Ok(guard) => {
            *INSTANCE_LOCK.lock().unwrap() = Some(guard);
            Ok(())
        }
        Err(_) => Err("Another instance of KUVPN is already running.".into()),
    }
}

/// Platform-relative path from the home directory to the kuvpn profile directory.
#[cfg(target_os = "linux")]
const PROFILE_SUBPATH: &str = ".local/share/kuvpn/profile";

#[cfg(target_os = "macos")]
const PROFILE_SUBPATH: &str = "Library/Application Support/kuvpn/profile";

#[cfg(target_os = "windows")]
const PROFILE_SUBPATH: &str = "AppData/Roaming/kuvpn/profile";

/// Returns a platform-appropriate user data directory for the Chrome profile.
///
/// - **Linux:** `~/.local/share/kuvpn/profile`
/// - **macOS:** `~/Library/Application Support/kuvpn/profile`
/// - **Windows:** `%USERPROFILE%\AppData\Roaming\kuvpn\profile`
///
/// Creates the directory if it does not already exist.
pub fn get_user_data_dir() -> Result<PathBuf, Box<dyn Error>> {
    let home_dir = env::var("HOME").or_else(|_| env::var("USERPROFILE"))?;
    let user_data_dir = PathBuf::from(&home_dir).join(PROFILE_SUBPATH);

    if !user_data_dir.exists() {
        std::fs::create_dir_all(&user_data_dir)?;
        log::info!("User data directory created at: {:?}", user_data_dir);
    }

    Ok(user_data_dir)
}

/// Completely removes the user data directory
pub fn wipe_user_data_dir() -> Result<(), Box<dyn Error>> {
    let path = get_user_data_dir()?;
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
        log::info!("Wiped profile directory: {:?}", path);
    }
    Ok(())
}

/// Escapes JavaScript strings to prevent injection.
pub fn js_escape(s: &str) -> String {
    s.replace("\\", "\\\\").replace("'", "\\'")
}
