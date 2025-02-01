mod args;
mod logger;

use args::Args;
use clap::Parser;
use headless_chrome::browser::default_executable;
use headless_chrome::{Browser, LaunchOptions};
use log::{error, info};
use logger::init_logger;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Duration;
use which::which;

/// Returns a platform-appropriate user data directory for the Chrome instance.
fn get_user_data_dir() -> Result<PathBuf, Box<dyn Error>> {
    let home_dir = env::var("HOME").or_else(|_| env::var("USERPROFILE"))?;

    #[cfg(target_os = "linux")]
    let base_path = ".local/share/kuvpn/profile";

    #[cfg(target_os = "macos")]
    let base_path = "Library/Application Support/kuvpn/profile";

    #[cfg(target_os = "windows")]
    let base_path = "AppData/Roaming/kuvpn/profile";

    let user_data_dir = PathBuf::from(format!("{}/{}", home_dir, base_path));

    if !user_data_dir.exists() {
        fs::create_dir_all(&user_data_dir)?;
        info!("User data directory created at: {:?}", user_data_dir);
    }

    Ok(user_data_dir)
}

/// Tries to locate `openconnect` in three steps:
/// 1. If `user_path` is an actual file on disk, use that.
/// 2. Otherwise, try searching on PATH (using `which`).
/// 3. Finally, check a few common sbin directories (e.g., `/sbin`).
fn locate_openconnect(user_path: &str) -> Option<PathBuf> {
    // 1. Check if `user_path` points directly to an existing file
    let candidate = Path::new(user_path);
    if candidate.exists() && candidate.is_file() {
        return Some(candidate.to_path_buf());
    }

    // 2. Otherwise, see if `which` can find `user_path` in PATH
    if let Ok(found) = which(user_path) {
        return Some(found);
    }

    // 3. Fallback to common sbin directories
    let fallback_dirs = ["/sbin", "/usr/sbin", "/usr/local/sbin"];
    for dir in &fallback_dirs {
        let path_in_dir = Path::new(dir).join("openconnect");
        if path_in_dir.exists() && path_in_dir.is_file() {
            return Some(path_in_dir);
        }
    }

    // Couldn't find `openconnect`
    None
}

/// Main entry point
fn main() -> ExitCode {
    let args = Args::parse();
    init_logger(&args.level);

    info!("Parsed arguments: {:?}", args);

    // If the user requested to clean session info
    if args.clean {
        let user_data_dir = match get_user_data_dir() {
            Ok(dir) => dir,
            Err(e) => {
                error!("Unable to get user data directory: {}", e);
                return ExitCode::FAILURE;
            }
        };

        info!("Cleaning user data directory: {:?}", user_data_dir);

        if user_data_dir.exists() {
            match fs::remove_dir_all(&user_data_dir) {
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

    // Create a browser to retrieve DSID (unless user just wants DSID or we skip)
    info!("Creating browser with agent: {}", args.agent);
    let browser = match create_browser(&args.agent) {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to create browser: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Fetch the DSID using the browser
    info!("Fetching DSID from URL: {}", args.url);
    let dsid = match fetch_dsid(&args.url, &browser) {
        Ok(cookie) => cookie,
        Err(e) => {
            error!("Error fetching DSID: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if args.dsid {
        // User only wants the DSID printed out
        info!("DSID retrieved: {}", dsid);
        println!("{}", dsid);
        return ExitCode::SUCCESS;
    }

    // We need to run openconnect, so locate the binary
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

    // Finally, execute openconnect
    if let Err(e) = execute_openconnect(dsid, args.url, &args.run_command, &openconnect_path) {
        error!("Error executing openconnect: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Create the browser (non-headless, with a blank page) using a user agent and a custom profile dir.
fn create_browser(agent: &str) -> Result<Browser, Box<dyn Error>> {
    let user_data_dir = get_user_data_dir()?;
    let user_agent = OsString::from(format!("--user-agent={agent}"));
    let body = OsString::from("--app=data:text/html,<html><body></body></html>");
    let window = OsString::from("--new-window");

    let mut options = LaunchOptions::default_builder();
    let mut launch_options = options
        .headless(false)
        .sandbox(false)
        .idle_browser_timeout(Duration::MAX)
        .window_size(Some((800, 800)))
        .args(vec![
            body.as_os_str(),
            window.as_os_str(),
            user_agent.as_os_str(),
        ])
        .user_data_dir(Some(user_data_dir));

    if let Ok(executable_path) = default_executable() {
        launch_options = launch_options.path(Some(executable_path));
    }

    Ok(Browser::new(launch_options.build()?)?)
}

/// Navigates to the given URL and loops until it finds the DSID cookie in the page's `document.cookie`.
fn fetch_dsid(url: &str, browser: &Browser) -> Result<String, Box<dyn Error>> {
    #[allow(deprecated)]
    let tab = browser.wait_for_initial_tab()?;

    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;

    info!("Navigating to URL: {}", url);

    loop {
        let script =
            "document.cookie.split('; ').find(row => row.startsWith('DSID='))?.split('=')[1];";
        let remote_object = tab.evaluate(script, true)?;

        if let Some(dsid_value) = remote_object.value {
            if let Some(dsid_string) = dsid_value.as_str() {
                tab.close_with_unload().expect("failed to close");
                info!("DSID value found: {}", dsid_string);
                return Ok(dsid_string.to_string());
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

/// Executes openconnect by elevating privileges (if needed) via a command like sudo/doas/pkexec.
/// We pass the full path to openconnect as `openconnect_path` (already resolved).
pub fn execute_openconnect(
    cookie_value: String,
    url: String,
    run_command: &Option<String>,
    openconnect_path: &Path,
) -> Result<(), Box<dyn Error>> {
    // Default list of privilege escalation tools
    let mut default_tools = vec!["doas", "sudo", "pkexec"];

    // If a custom run command is provided, check if it's available and prioritize it
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

    // Find the first available escalation tool
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
