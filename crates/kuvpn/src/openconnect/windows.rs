//! Windows-specific OpenConnect process management.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use runas::Command as AdminCommand;
use sysinfo::System;

use super::VpnProcess;

// ── Private helpers ───────────────────────────────────────────────────────────

/// Sends an elevated `taskkill /PID` and returns `true` if openconnect stopped.
fn try_taskkill_pid(pid: u32) -> bool {
    let mut cmd = AdminCommand::new("taskkill");
    cmd.show(false)
        .arg("/F")
        .arg("/T")
        .arg("/PID")
        .arg(pid.to_string());
    let _ = cmd.status();
    std::thread::sleep(std::time::Duration::from_millis(500));
    !is_openconnect_running()
}

// ── Platform implementations ──────────────────────────────────────────────────

/// Terminates the specific openconnect process that we launched (elevation required).
/// Falls back to a non-elevated attempt if the PID is unknown or the kill fails.
pub(super) fn kill_vpn_process(pid: Option<u32>) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command as StdCommand;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    log::info!("Requesting Admin elevation to stop OpenConnect...");

    if let Some(p) = pid {
        if try_taskkill_pid(p) {
            log::info!("OpenConnect (PID {}) terminated successfully", p);
            return Ok(());
        }
        log::warn!(
            "Elevated kill of PID {} failed, trying non-elevated fallback...",
            p
        );
    } else {
        log::warn!("OpenConnect PID unknown, trying non-elevated fallback...");
    }

    // Last resort: non-elevated taskkill by name (may fail for elevated openconnect).
    let _ = StdCommand::new("taskkill")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/F", "/IM", "openconnect.exe"])
        .status();

    Ok(())
}

pub(super) fn vpn_process_alive(thread_finished: &Arc<AtomicBool>) -> bool {
    if !thread_finished.load(Ordering::SeqCst) {
        return true;
    }
    is_openconnect_running()
}

/// Executes openconnect on Windows with admin elevation (via `runas`).
pub(super) fn execute(
    cookie_value: String,
    url: String,
    openconnect_path: &Path,
) -> anyhow::Result<VpnProcess> {
    log::info!("Requesting Admin elevation for OpenConnect...");

    let oc_path_str = openconnect_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("openconnect path contains invalid UTF-8"))?;

    let mut cmd = AdminCommand::new(oc_path_str);
    cmd.show(false)
        .arg("--protocol")
        .arg("nc")
        .arg("-C")
        .arg(format!("DSID={}", cookie_value))
        .arg(url);

    // `runas` blocks until the process exits, so we run it on a background thread.
    // The `thread_finished` flag signals when the process has ended.
    let thread_finished = Arc::new(AtomicBool::new(false));
    let pid = Arc::new(AtomicU32::new(0));

    let tf_main = Arc::clone(&thread_finished);
    std::thread::spawn(move || {
        match cmd.status() {
            Ok(status) if !status.success() => log::error!("OpenConnect exited with failure."),
            Err(e) => log::error!("Failed to run elevated OpenConnect: {}", e),
            _ => {}
        }
        tf_main.store(true, Ordering::SeqCst);
    });

    // Discover the PID in a second thread — `runas` gives us no handle, so we poll
    // the process list until openconnect appears (or the main thread finishes first).
    let tf_pid = Arc::clone(&thread_finished);
    let pid_clone = Arc::clone(&pid);
    std::thread::spawn(move || {
        for _ in 0..20 {
            // Stop searching if the process already exited (UAC cancelled, instant failure).
            if tf_pid.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Some(found) = get_openconnect_pid() {
                pid_clone.store(found, Ordering::SeqCst);
                log::info!("[*] OpenConnect process found (PID: {})", found);
                return;
            }
        }
        log::warn!("[!] Could not discover OpenConnect PID after launch");
    });

    Ok(VpnProcess::Windows {
        thread_finished,
        pid,
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

/// Gracefully terminates a process by PID (with admin elevation).
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
