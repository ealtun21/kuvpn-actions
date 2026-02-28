//! # KUVPN CLI
//!
//! A clean terminal interface for connecting to Koc University's VPN.

mod args;
mod credentials;

use args::Args;
use clap::Parser;
use console::Style;
use credentials::{format_duration, CliCredentialsProvider};
use indicatif::{ProgressBar, ProgressStyle};
use kuvpn::{
    init_logger, run_login_and_get_dsid, ConnectionStatus, LoginConfig, ParsedLog, SessionConfig,
    VpnSession,
};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ── Terminal styles ───────────────────────────────────────────────────────────

struct CliStyles {
    green: Style,
    red: Style,
    dim: Style,
    bold: Style,
    yellow: Style,
}

impl CliStyles {
    fn new() -> Self {
        Self {
            green: Style::new().green().bold(),
            red: Style::new().red().bold(),
            dim: Style::new().dim(),
            bold: Style::new().bold(),
            yellow: Style::new().yellow().bold(),
        }
    }
}

// ── Log message handling ──────────────────────────────────────────────────────

/// Processes one parsed log entry.
///
/// Returns `true` if the message was handled (no further processing needed).
/// Mutates `spinner_active` and `connection_start` in place.
fn handle_log(
    parsed: &ParsedLog,
    spinner: &ProgressBar,
    spinner_active: &mut bool,
    connection_start: &mut Option<Instant>,
    styles: &CliStyles,
    interface_name: &str,
) -> bool {
    let msg = parsed.message.as_str();

    match msg {
        "Accessing campus gateway..." => {
            if !*spinner_active {
                spinner.enable_steady_tick(Duration::from_millis(80));
                *spinner_active = true;
            }
            spinner.set_message("Accessing campus gateway...");
            return true;
        }
        "Initializing tunnel..." => {
            clear_spinner(spinner, spinner_active);
            eprintln!(
                "  {} Accessing campus gateway...",
                styles.green.apply_to("✓")
            );
            eprintln!("  {} Initializing tunnel...", styles.green.apply_to("✓"));
            return true;
        }
        "VPN interface already active, monitoring..." => {
            clear_spinner(spinner, spinner_active);
            eprintln!(
                "  {} VPN already active, monitoring connection",
                styles.yellow.apply_to("~"),
            );
            *connection_start = Some(Instant::now());
            return true;
        }
        "Connected." => {
            clear_spinner(spinner, spinner_active);
            *connection_start = Some(Instant::now());
            print_connected(interface_name, styles);
            eprintln!("    {}", styles.dim.apply_to("Press Ctrl+C to disconnect"));
            return true;
        }
        "Disconnecting..." => {
            if *spinner_active {
                spinner.finish_and_clear();
            }
            spinner.set_message("Disconnecting...");
            spinner.enable_steady_tick(Duration::from_millis(80));
            *spinner_active = true;
            return true;
        }
        "Disconnected." => {
            clear_spinner(spinner, spinner_active);
            let duration = connection_start
                .map(|s| format!(" (session: {})", format_duration(s.elapsed())))
                .unwrap_or_default();
            eprintln!(
                "  {} Disconnected{}",
                styles.dim.apply_to("●"),
                styles.dim.apply_to(duration),
            );
            return true;
        }
        _ if msg.ends_with("requires a password. Prompting...") => return true,
        _ => {}
    }

    if parsed.level == log::Level::Error {
        clear_spinner(spinner, spinner_active);
        eprintln!("  {} {}", styles.red.apply_to("✗"), msg);
        if msg.contains("Full Auto mode unable to complete login")
            || msg.contains("Could not find a handler")
        {
            eprintln!();
            eprintln!(
                "  {} Try the following:",
                styles.bold.apply_to("Suggestions:")
            );
            eprintln!(
                "    {} Switch to manual mode: {}",
                styles.dim.apply_to("•"),
                styles.bold.apply_to("--no-auto-login --disable-headless"),
            );
            eprintln!(
                "    {} Wipe session cache:    {}",
                styles.dim.apply_to("•"),
                styles.bold.apply_to("--clean"),
            );
        }
        return true;
    }

    // Pass through to the logger for non-status messages.
    match parsed.level {
        log::Level::Warn => log::warn!("{}", msg),
        _ => log::info!("{}", msg),
    }
    true
}

fn clear_spinner(spinner: &ProgressBar, active: &mut bool) {
    if *active {
        spinner.finish_and_clear();
        *active = false;
    }
}

fn print_connected(interface_name: &str, styles: &CliStyles) {
    #[cfg(unix)]
    {
        let iface = kuvpn::get_vpn_interface_name(interface_name)
            .unwrap_or_else(|| interface_name.to_string());
        eprintln!(
            "  {} Connected to KU VPN {}",
            styles.green.apply_to("✓"),
            styles.dim.apply_to(format!("(interface: {})", iface)),
        );
    }
    #[cfg(not(unix))]
    {
        let _ = interface_name;
        eprintln!("  {} Connected to KU VPN", styles.green.apply_to("✓"));
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    // VPN helper mode: invoked by the app itself under elevation to manage
    // OpenConnect's lifecycle (single UAC prompt per connection).
    // Must run before clap parses args (it would reject --vpn-helper as unknown).
    #[cfg(windows)]
    if let Some(code) = kuvpn::run_vpn_helper_if_requested() {
        return ExitCode::from(code as u8);
    }

    let args = Args::parse();
    init_logger(args.level.clone().into());

    let styles = CliStyles::new();

    eprintln!(
        "{} {}",
        styles.bold.apply_to("KUVPN"),
        styles
            .dim
            .apply_to(format!("v{}", env!("CARGO_PKG_VERSION"))),
    );

    if let Err(e) = kuvpn::utils::ensure_single_instance() {
        eprintln!("  {} {}", styles.red.apply_to("✗"), e);
        return ExitCode::FAILURE;
    }

    if args.clean {
        return match kuvpn::utils::wipe_user_data_dir() {
            Ok(_) => {
                eprintln!("  {} Session data wiped", styles.green.apply_to("✓"));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!(
                    "  {} Failed to wipe session data: {}",
                    styles.red.apply_to("✗"),
                    e
                );
                ExitCode::FAILURE
            }
        };
    }

    if args.get_dsid {
        return run_get_dsid(&args, &styles);
    }

    run_vpn_session(&args, &styles)
}

fn run_get_dsid(args: &Args, styles: &CliStyles) -> ExitCode {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(spinner_style());
    spinner.set_message("Retrieving DSID...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let config = LoginConfig {
        headless: !args.disable_headless,
        url: args.url.clone(),
        domain: args.domain.clone(),
        user_agent: "Mozilla/5.0".to_string(),
        no_auto_login: args.no_auto_login,
        email: args.email.clone(),
    };

    match run_login_and_get_dsid(
        &config,
        &kuvpn::utils::TerminalCredentialsProvider,
        None,
        None,
    ) {
        Ok(dsid) => {
            spinner.finish_and_clear();
            println!("{}", dsid);
            ExitCode::SUCCESS
        }
        Err(e) => {
            spinner.finish_and_clear();
            eprintln!("  {} Login failed: {}", styles.red.apply_to("✗"), e);
            ExitCode::FAILURE
        }
    }
}

fn run_vpn_session(args: &Args, styles: &CliStyles) -> ExitCode {
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

    let spinner = Arc::new(ProgressBar::new_spinner());
    spinner.set_style(spinner_style());

    let session = VpnSession::new(config);
    let (log_tx, log_rx) = crossbeam_channel::unbounded();
    session.set_logs_tx(log_tx);

    let provider = Arc::new(CliCredentialsProvider {
        spinner: Arc::clone(&spinner),
    });
    let _join_handle = session.connect(provider);

    let cancel = session.cancel_token();
    ctrlc::set_handler(move || cancel.cancel()).ok();

    let mut connection_start: Option<Instant> = None;
    let mut spinner_active = false;

    loop {
        while let Ok(raw) = log_rx.try_recv() {
            if let Some(parsed) = ParsedLog::parse(&raw) {
                handle_log(
                    &parsed,
                    &spinner,
                    &mut spinner_active,
                    &mut connection_start,
                    styles,
                    &args.interface_name,
                );
            } else {
                log::info!("{}", raw);
            }
        }

        if session.is_finished() {
            clear_spinner(&spinner, &mut spinner_active);
            return if session.status() == ConnectionStatus::Error {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            };
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .template("{spinner:.cyan} {msg}")
        .unwrap()
}
