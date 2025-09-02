//! # VPN Connection Tool
//!
//! This application fetches a DSID cookie from a specified URL using a non-headless
//! Chrome browser and then uses that DSID to establish a VPN connection via the
//! `openconnect` command. It supports cleaning session data, specifying a custom
//! user agent, and executing with elevated privileges using tools like `sudo`/`doas`/`pkexec`.

mod args;
mod dsid;
mod logger;

use args::Args;
use clap::Parser;
use log::{error, info};
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use which::which;

use crate::dsid::run_login_and_get_dsid;
use crate::logger::init_logger;

/// Attempts to locate the `openconnect` executable.
///
/// The search is performed in three steps:
/// 1. Check if the provided `user_path` directly points to an existing file.
/// 2. Use the system's PATH (via the `which` crate) to locate `openconnect`.
/// 3. Fallback: Check common sbin directories (e.g., `/sbin`, `/usr/sbin`, `/usr/local/sbin`).
///
/// # Arguments
///
/// * `user_path` - A string slice that holds the user-provided path to `openconnect`.
///
/// # Returns
///
/// * `Some(PathBuf)` if the executable is found.
/// * `None` if the executable cannot be located.
fn locate_openconnect(user_path: &str) -> Option<PathBuf> {
    // Step 1: Check if `user_path` directly points to an existing file.
    let candidate = Path::new(user_path);
    if candidate.exists() && candidate.is_file() {
        return Some(candidate.to_path_buf());
    }

    // Step 2: Attempt to locate `openconnect` using the system's PATH.
    if let Ok(found) = which(user_path) {
        return Some(found);
    }

    // Step 3: Fallback to searching in common sbin directories.
    let fallback_dirs = ["/sbin", "/usr/sbin", "/usr/local/sbin"];
    for dir in &fallback_dirs {
        let path_in_dir = Path::new(dir).join("openconnect");
        if path_in_dir.exists() && path_in_dir.is_file() {
            return Some(path_in_dir);
        }
    }

    // Return None if `openconnect` could not be found.
    None
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
        info!("User data directory created at: {:?}", user_data_dir);
    }

    Ok(user_data_dir)
}

/// The main entry point of the application.
///
/// This function performs the following steps:
/// 1. Parses command-line arguments and initializes logging.
/// 2. If the user requested a session clean-up, it removes the session data directory.
/// 3. Creates a browser instance to retrieve the DSID cookie from the given URL.
/// 4. If only the DSID is required, it prints the value and exits.
/// 5. Otherwise, locates the `openconnect` executable and executes it with elevated privileges.
///
/// # Returns
///
/// An `ExitCode` indicating success (`ExitCode::SUCCESS`) or failure (`ExitCode::FAILURE`).
fn main() -> ExitCode {
    // Parse command-line arguments.
    let args = Args::parse();

    init_logger(&args.level);

    // Handle the clean session option.
    if args.clean {
        let user_data_dir = match get_user_data_dir() {
            Ok(dir) => dir,
            Err(e) => {
                error!("Unable to get user data directory: {}", e);
                return ExitCode::FAILURE;
            }
        };

        println!("[*] Cleaning user data directory: {:?}", user_data_dir);

        if user_data_dir.exists() {
            match std::fs::remove_dir_all(&user_data_dir) {
                Ok(_) => {
                    info!("Session information successfully removed.");
                    return ExitCode::SUCCESS;
                }
                Err(e) => {
                    error!("Failed to remove session information: {}", e);
                    return ExitCode::FAILURE;
                }
            }
        } else {
            info!("No session information found.");
            return ExitCode::FAILURE;
        }
    }

    let dsid = match run_login_and_get_dsid(!args.disable_headless, &args.url, &args.domain) {
        Ok(dsid) => dsid,
        Err(e) => {
            error!("Login process failed: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if args.get_dsid == true {
        println!("{}", dsid);
        return ExitCode::SUCCESS;
    }

    // Locate the `openconnect` executable.
    let openconnect_path = match locate_openconnect(&args.openconnect_path) {
        Some(path) => {
            info!("OpenConnect located at: {}", path.display());
            path
        }
        None => {
            error!(
                "Cannot locate openconnect (checked path '{}', PATH, and /sbin). \
                 Please install it or specify --openconnect-path <absolute_path>.",
                args.openconnect_path
            );
            return ExitCode::FAILURE;
        }
    };

    // Execute `openconnect` with the retrieved DSID and specified URL.
    if let Err(e) = execute_openconnect(dsid, args.url, &args.run_command, &openconnect_path) {
        error!("Error executing openconnect: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Executes the `openconnect` command with elevated privileges using an available escalation tool.
///
/// The function supports a custom run command if provided; otherwise, it defaults to a list of
/// common escalation tools (`doas`, `sudo`, `pkexec`). It constructs the appropriate command-line
/// arguments and executes `openconnect` with the DSID and VPN URL.
///
/// # Arguments
///
/// * `cookie_value` - The DSID cookie value to be passed to `openconnect`.
/// * `url` - The VPN URL to connect to.
/// * `run_command` - An optional custom command for elevated privilege execution.
/// * `openconnect_path` - The resolved path to the `openconnect` executable.
///
/// # Returns
///
/// A `Result` indicating success or failure of the command execution.
pub fn execute_openconnect(
    cookie_value: String,
    url: String,
    run_command: &Option<String>,
    openconnect_path: &Path,
) -> Result<(), Box<dyn Error>> {
    // Default list of privilege escalation tools.
    let mut default_tools = vec!["doas", "sudo", "pkexec"];

    // If a custom run command is provided, check its availability and prioritize it.
    if let Some(custom_command) = run_command {
        info!("Custom run command provided: {}", custom_command);

        if which(custom_command).is_ok() {
            info!("Custom command found: {}", custom_command);
            default_tools.insert(0, custom_command.as_str());
        } else {
            println!(
                "Custom command '{}' not found, falling back to default tools.",
                custom_command
            );
            info!(
                "Custom command '{}' not found, using default tools.",
                custom_command
            );
        }
    } else {
        info!("No custom run command provided, defaulting to built-in tools.");
    }

    // Identify the first available escalation tool from the list.
    info!("Checking for available tools/commands: {:?}", default_tools);
    let command_to_run = default_tools
        .iter()
        .find_map(|&tool| which(tool).ok().map(|_| tool))
        .ok_or("No available tool for running openconnect (sudo/doas/pkexec not found)")?;

    println!(
        "Running openconnect using {} for elevated privileges or execution",
        command_to_run
    );

    info!(
        "Command to run: {} {:?} --protocol nc -C DSID={} {}",
        command_to_run,
        openconnect_path.display(),
        cookie_value,
        url
    );

    // Execute the command with the constructed arguments.
    // Example: sudo /path/to/openconnect --protocol nc -C DSID=abcd https://vpn.ku.edu.tr
    Command::new(command_to_run)
        .arg(openconnect_path)
        .arg("--protocol")
        .arg("nc")
        .arg("-C")
        .arg(format!("DSID={}", cookie_value))
        .arg(url)
        .status()?;

    Ok(())
}