//! # VPN Connection Tool
//!
//! This application fetches a DSID cookie from a specified URL using a non-headless
//! Chrome browser and then uses that DSID to establish a VPN connection via the
//! `openconnect` command. It supports cleaning session data, specifying a custom
//! user agent, and executing with elevated privileges using tools like `sudo`/`doas`/`pkexec`.

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
fn get_user_data_dir() -> Result<PathBuf, Box<dyn Error>> {
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
        fs::create_dir_all(&user_data_dir)?;
        info!("User data directory created at: {:?}", user_data_dir);
    }

    Ok(user_data_dir)
}

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

    info!("Parsed arguments: {:?}", args);

    // Handle the clean session option.
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


    if let Some(dsid) = args.set_dsid.as_ref().filter(|x| !x.is_empty()) {
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

        if let Err(e) = execute_openconnect(dsid.to_string(), args.url, &args.run_command, &openconnect_path) {
            error!("Error executing openconnect: {}", e);
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS
    }


    // Create a browser instance with the provided user agent.
    info!("Creating browser with agent: {}", args.agent);
    let browser = match create_browser(&args.agent) {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to create browser: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Fetch the DSID cookie from the specified URL.
    info!("Fetching DSID from URL: {}", args.url);
    let dsid = match fetch_dsid(&args.url, &browser) {
        Ok(cookie) => cookie,
        Err(e) => {
            error!("Error fetching DSID: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Release the browser resources.
    drop(browser);

    // If only the DSID is requested, print it and exit.
    if args.get_dsid {
        info!("DSID retrieved: {}", dsid);
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

/// Creates a non-headless browser instance configured with a blank page, a custom user agent,
/// and a dedicated user data directory.
///
/// The browser is launched with the following settings:
/// - **Non-headless mode** to allow for potential UI interactions.
/// - **A blank app window** containing a minimal HTML page.
/// - **Custom user agent** as specified by the `agent` parameter.
/// - **Custom user data directory** for isolated session data.
/// - **Custom window size** of 800x800 pixels.
/// - **Sandbox disabled** and an infinite idle timeout.
///
/// # Arguments
///
/// * `agent` - A string slice representing the desired user agent.
///
/// # Returns
///
/// A `Result` containing the configured `Browser` instance on success, or an error on failure.
fn create_browser(agent: &str) -> Result<Browser, Box<dyn Error>> {
    // Retrieve the user data directory.
    let user_data_dir = get_user_data_dir()?;

    // Build the command-line arguments for launching the browser.
    let user_agent = OsString::from(format!("--user-agent={agent}"));
    let body = OsString::from("--app=data:text/html,<html><body></body></html>");
    let window = OsString::from("--new-window");

    // Configure the browser launch options.
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

    // If the default Chrome executable is available, specify its path.
    if let Ok(executable_path) = default_executable() {
        launch_options = launch_options.path(Some(executable_path));
    }

    // Build and return the Browser instance.
    Ok(Browser::new(launch_options.build()?)?)
}

/// Navigates to the specified URL using the provided browser instance and retrieves the DSID cookie.
///
/// The function uses the browser's initial tab to load the URL and repeatedly evaluates a JavaScript
/// snippet until the DSID cookie is found. Once retrieved, the tab is closed and the DSID is returned.
///
/// # Arguments
///
/// * `url` - A string slice containing the URL to navigate to.
/// * `browser` - A reference to the `Browser` instance to use for navigation.
///
/// # Returns
///
/// A `Result` containing the DSID as a `String` on success, or an error if the DSID cannot be obtained.
fn fetch_dsid(url: &str, browser: &Browser) -> Result<String, Box<dyn Error>> {
    // Use the initial tab provided by the browser.
    #[allow(deprecated)]
    let tab = browser.wait_for_initial_tab()?;

    // Navigate to the target URL and wait for the page to load.
    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;

    info!("Navigating to URL: {}", url);

    // Continuously check for the DSID cookie.
    loop {
        // JavaScript snippet to extract the DSID cookie value.
        let script =
            "document.cookie.split('; ').find(row => row.startsWith('DSID='))?.split('=')[1];";
        let remote_object = tab.evaluate(script, true)?;

        // If the DSID cookie is found, close the tab and return the value.
        if let Some(dsid_value) = remote_object.value {
            if let Some(dsid_string) = dsid_value.as_str() {
                tab.close_with_unload().expect("failed to close");
                info!("DSID value found: {}", dsid_string);
                return Ok(dsid_string.to_string());
            }
        }

        // Wait briefly before checking again.
        thread::sleep(Duration::from_millis(100));
    }
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
