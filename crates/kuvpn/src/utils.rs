use rpassword::read_password;
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::path::PathBuf;

/// Trait for providing credentials and user input.
pub trait CredentialsProvider {
    fn request_text(&self, msg: &str) -> String;
    fn request_password(&self, msg: &str) -> String;
}

/// A implementation of `CredentialsProvider` for terminal input.
pub struct TerminalCredentialsProvider;

impl CredentialsProvider for TerminalCredentialsProvider {
    fn request_text(&self, msg: &str) -> String {
        prompt_text(msg)
    }

    fn request_password(&self, msg: &str) -> String {
        prompt_password(msg)
    }
}

/// Returns a platform-appropriate user data directory for the Chrome instance.
///
/// The directory path is constructed based on the operating system:
/// - **Linux:** `~/.local/share/kuvpn/profile`
/// - **macOS:** `~/Library/Application Support/kuvpn/profile`
/// - **Windows:** `%USERPROFILE%\AppData\Roaming\kuvpn\profile`
///
/// If the directory does not exist, it is created.
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined or if the directory cannot be created.
pub fn get_user_data_dir() -> Result<PathBuf, Box<dyn Error>> {
    // Determine the user's home directory from environment variables.
    let home_dir = env::var("HOME").or_else(|_| env::var("USERPROFILE"))?;

    // Select the appropriate base path for the current operating system.
    #[cfg(target_os = "linux")]
    let base_path = ".local/share/kuvpn/profile";

    #[cfg(target_os = "macos")]
    let base_path = "Library/Application Support/kuvpn/profile";

    #[cfg(target_os = "windows")]
    let base_path = "AppData/Roaming/kuvpn/profile";

    // Construct the full user data directory path.
    let user_data_dir = PathBuf::from(format!("{}/{}", home_dir, base_path));

    // Create the directory if it does not exist.
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

/// Prompts the user for text input.
pub fn prompt_text(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_owned()
}

/// Prompts the user for password input (hidden).
pub fn prompt_password(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().unwrap();
    read_password().unwrap()
}
