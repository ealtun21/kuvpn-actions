//! Windows-specific OpenConnect process management.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use runas::Command as AdminCommand;
use sysinfo::System;

use super::VpnProcess;

// ── Private helpers ───────────────────────────────────────────────────────────

/// Returns a string unique within this process lifetime (PID + nanosecond timestamp).
fn unique_id() -> String {
    use std::time::SystemTime;
    let t = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{}", std::process::id(), t)
}

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

pub(super) fn vpn_process_alive(thread_finished: &Arc<AtomicBool>) -> bool {
    if !thread_finished.load(Ordering::SeqCst) {
        return true;
    }
    is_openconnect_running()
}

/// Starts OpenConnect on Windows via an elevated helper process (single UAC prompt).
///
/// Instead of elevating openconnect directly, we elevate a copy of ourselves with
/// `--vpn-helper <stop-file> <oc-path> <url> <dsid>`.  The helper starts openconnect
/// as its own child (inheriting the elevated token) and monitors the stop-signal file.
/// When the parent creates that file, the helper kills openconnect and exits — all
/// without a second UAC prompt.
pub(super) fn execute(
    cookie_value: String,
    url: String,
    openconnect_path: &Path,
) -> anyhow::Result<VpnProcess> {
    // Path to this binary — used as the helper executable.
    let helper_exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Cannot locate helper binary: {}", e))?;

    let oc_path_str = openconnect_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("openconnect path contains invalid UTF-8"))?;

    // Unique per-session stop-signal file in the user's temp directory.
    let stop_file = std::env::temp_dir().join(format!("kuvpn-stop-{}.signal", unique_id()));

    log::info!("Requesting Admin elevation for OpenConnect (single prompt)...");

    let mut cmd = AdminCommand::new(&helper_exe);
    cmd.show(false)
        .arg("--vpn-helper")
        .arg(&stop_file)
        .arg(oc_path_str)
        .arg(&url)
        .arg(&cookie_value);

    // `runas` blocks until the helper exits, so we run it on a background thread.
    let thread_finished = Arc::new(AtomicBool::new(false));
    let tf_clone = Arc::clone(&thread_finished);
    std::thread::spawn(move || {
        match cmd.status() {
            Ok(status) if !status.success() => {
                log::error!("VPN helper exited with failure.")
            }
            Err(e) => log::error!("Failed to run elevated helper: {}", e),
            _ => {}
        }
        tf_clone.store(true, Ordering::SeqCst);
    });

    Ok(VpnProcess::Windows {
        thread_finished,
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

/// Returns `true` if the named VPN interface reports as "Connected" in netsh.
pub fn is_vpn_interface_up(interface_name: &str) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::{Command as StdCommand, Stdio as StdStdio};
    const CREATE_NO_WINDOW: u32 = 0x08000000;

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
