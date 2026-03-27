//! Windows-specific OpenConnect process management.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use runas::Command as AdminCommand;
use sysinfo::System;

use super::VpnProcess;

const CREATE_NO_WINDOW: u32 = 0x08000000;

// ── Platform implementations ──────────────────────────────────────────────────

/// Signals the elevated helper to stop OpenConnect by creating the stop-signal file.
/// The helper polls for this file every 200 ms and terminates OpenConnect when it appears.
/// No second UAC prompt is needed because the helper owns the OpenConnect child process.
pub(super) fn kill_vpn_process(stop_file: &Path) -> anyhow::Result<()> {
    log::info!("Sending stop signal to VPN helper...");
    std::fs::File::create(stop_file)
        .map_err(|e| anyhow::anyhow!("Failed to create stop signal file: {}", e))?;
    Ok(())
}

pub(super) fn vpn_process_alive(
    thread_finished: &Arc<AtomicBool>,
    thread_failed_reason: &Arc<Mutex<Option<String>>>,
) -> bool {
    if thread_failed_reason
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
    {
        return false;
    }
    if !thread_finished.load(Ordering::SeqCst) {
        return true;
    }
    is_openconnect_running()
}

/// Starts OpenConnect on Windows via an elevated helper process (single UAC prompt).
///
/// Instead of elevating openconnect directly, we elevate a copy of ourselves with
/// `--vpn-helper <oc-path> <url> <dsid> <parent-pid> <stop-file>`.  The helper
/// starts openconnect as its own child (inheriting the elevated token) and monitors
/// the stop-signal file.  When the parent creates that file, the helper kills
/// openconnect and exits — all without a second UAC prompt.
pub(super) fn execute(
    cookie_value: String,
    url: String,
    openconnect_path: &Path,
    full_tunnel: bool,
) -> anyhow::Result<VpnProcess> {
    // Path to this binary — used as the helper executable.
    let helper_exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Cannot locate helper binary: {}", e))?;

    let oc_path_str = openconnect_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("openconnect path contains invalid UTF-8"))?;

    let parent_pid = std::process::id();
    let stop_file = std::env::temp_dir().join(format!("kuvpn-stop-{}.signal", parent_pid));

    // Pass the stop-file path to the helper using forward slashes instead of
    // backslashes.  The runas crate doubles every backslash inside quoted
    // arguments (to satisfy CommandLineToArgvW), which corrupts Windows paths
    // that contain spaces (e.g. "C:\Users\John Doe\...").  Forward slashes are
    // left untouched and are accepted by all Windows file-system APIs, so the
    // helper can reconstruct the exact path from the argument.
    let stop_file_arg = stop_file.to_string_lossy().replace('\\', "/");

    log::info!("Requesting Admin elevation for OpenConnect (single prompt)...");

    let mut cmd = AdminCommand::new(&helper_exe);
    cmd.show(false)
        .arg("--vpn-helper")
        .arg(oc_path_str)
        .arg(&url)
        .arg(&cookie_value)
        .arg(parent_pid.to_string())
        .arg(&stop_file_arg);

    if full_tunnel {
        cmd.arg("--full-tunnel");
    }

    // `runas` blocks until the helper exits, so we run it on a background thread.
    let thread_finished = Arc::new(AtomicBool::new(false));
    let thread_failed_reason: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let tf_clone = Arc::clone(&thread_finished);
    let fail_clone = Arc::clone(&thread_failed_reason);
    std::thread::spawn(move || {
        let reason = match cmd.status() {
            Ok(status) if !status.success() => Some(
                "VPN helper exited with a non-zero status — UAC may have been denied or openconnect failed to start".to_string(),
            ),
            Err(e) => Some(format!("Failed to run elevated VPN helper: {e}")),
            _ => None,
        };
        if let Some(ref msg) = reason {
            log::error!("{}", msg);
            if let Ok(mut guard) = fail_clone.lock() {
                *guard = Some(msg.clone());
            }
        }
        tf_clone.store(true, Ordering::SeqCst);
    });

    Ok(VpnProcess::Windows {
        thread_finished,
        thread_failed_reason,
        stop_file,
    })
}

// ── Public platform functions ─────────────────────────────────────────────────

/// Checks whether an openconnect process is currently running.
pub fn is_openconnect_running() -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.processes().values().any(|p| {
        p.name()
            .to_string_lossy()
            .to_lowercase()
            .contains("openconnect")
    })
}

/// Returns the PID of the running openconnect process, if any.
pub fn get_openconnect_pid() -> Option<u32> {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.processes().iter().find_map(|(pid, process)| {
        process
            .name()
            .to_string_lossy()
            .to_lowercase()
            .contains("openconnect")
            .then(|| pid.as_u32())
    })
}

/// Terminates a process by PID with admin elevation.
/// Used as a last-resort cleanup; normal disconnect goes through `kill_vpn_process`.
pub fn kill_process(pid: u32) -> anyhow::Result<()> {
    let mut cmd = AdminCommand::new("taskkill");
    cmd.show(false)
        .arg("/F")
        .arg("/T")
        .arg("/PID")
        .arg(pid.to_string());
    cmd.status()?;
    Ok(())
}

/// Force-kills a process by PID using taskkill (non-elevated, for browser processes).
pub(crate) fn kill_browser_process(pid: u32) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("taskkill")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/F", "/PID", &pid.to_string()])
        .status();
}

/// Returns `None` — Windows does not support interface-name-based VPN detection.
pub fn get_vpn_interface_name(_configured_name: &str) -> Option<String> {
    None
}

/// Returns `true` if the named VPN interface reports as "Connected" in netsh.
pub fn is_vpn_interface_up(interface_name: &str) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::{Command as StdCommand, Stdio as StdStdio};

    let output = StdCommand::new("netsh")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["interface", "show", "interface"])
        .stdout(StdStdio::piped())
        .stderr(StdStdio::null())
        .output();

    output.is_ok_and(|out| {
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|line| line.contains(interface_name) && line.contains("Connected"))
    })
}
