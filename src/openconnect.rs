use log::info;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

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
pub fn locate_openconnect(user_path: &str) -> Option<PathBuf> {
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
