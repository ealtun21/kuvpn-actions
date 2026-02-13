//! # VPN Connection Tool
//!
//! This application fetches a DSID cookie from a specified URL using a browser
//! and then uses that DSID to establish a VPN connection via the `openconnect` command.

mod args;

use args::Args;
use clap::Parser;
use kuvpn::{init_logger, run_login_and_get_dsid, ParsedLog, SessionConfig, VpnSession, ConnectionStatus};
use log::{error, info};
use std::process::ExitCode;
use std::sync::Arc;

/// The main entry point of the application.
fn main() -> ExitCode {
    // Parse command-line arguments.
    let args = Args::parse();
    init_logger(args.level.clone().into());

    // Ensure only one instance is running
    if let Err(e) = kuvpn::utils::ensure_single_instance() {
        error!("{}", e);
        return ExitCode::FAILURE;
    }

    // Handle the clean session option.
    if args.clean {
        match kuvpn::utils::wipe_user_data_dir() {
            Ok(_) => {
                info!("Session information successfully removed.");
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                error!("Failed to remove session information: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    if args.get_dsid {
        match run_login_and_get_dsid(
            !args.disable_headless,
            &args.url,
            &args.domain,
            "Mozilla/5.0",
            args.no_auto_login,
            args.email,
            &kuvpn::utils::TerminalCredentialsProvider,
            None,
            None,
        ) {
            Ok(dsid) => {
                println!("{}", dsid);
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                error!("Login process failed: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    let config = SessionConfig {
        url: args.url.clone(),
        domain: args.domain.clone(),
        user_agent: "Mozilla/5.0".to_string(),
        headless: !args.disable_headless,
        no_auto_login: args.no_auto_login,
        email: args.email.clone(),
        openconnect_path: args.openconnect_path.clone(),
        escalation_tool: args.run_command.clone(),
        interface_name: args.interface_name.clone(),
    };

    let session = VpnSession::new(config);
    let (log_tx, log_rx) = crossbeam_channel::unbounded();
    session.set_logs_tx(log_tx);

    let provider = Arc::new(kuvpn::utils::TerminalCredentialsProvider);
    let _join_handle = session.connect(provider);

    loop {
        while let Ok(log_msg) = log_rx.try_recv() {
            if let Some(parsed) = ParsedLog::parse(&log_msg) {
                match parsed.level {
                    log::Level::Error => error!("{}", parsed.message),
                    log::Level::Warn => log::warn!("{}", parsed.message),
                    _ => info!("{}", parsed.message),
                }
            } else {
                info!("{}", log_msg);
            }
        }

        if session.is_finished() {
            if session.status() == ConnectionStatus::Error {
                if let Some(err) = session.last_error() {
                    error!("Session Error: {}", err);
                }
                return ExitCode::FAILURE;
            }
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    ExitCode::SUCCESS
}
