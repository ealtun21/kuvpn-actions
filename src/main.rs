//! # VPN Connection Tool
//!
//! This application fetches a DSID cookie from a specified URL using a browser
//! and then uses that DSID to establish a VPN connection via the `openconnect` command.

mod args;
mod browser;
mod dsid;
mod handlers;
mod logger;
mod openconnect;
mod utils;

use args::Args;
use clap::Parser;
use dsid::run_login_and_get_dsid;
use log::{error, info};
use logger::init_logger;
use openconnect::{execute_openconnect, locate_openconnect};
use std::process::ExitCode;
use utils::get_user_data_dir;

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

    let dsid = match run_login_and_get_dsid(
        !args.disable_headless,
        &args.url,
        &args.domain,
        "Mozilla/5.0",
    ) {
        Ok(dsid) => dsid,
        Err(e) => {
            error!("Login process failed: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if args.get_dsid {
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
                Please install it or specify --openconnect-path .",
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
