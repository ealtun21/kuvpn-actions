//! VPN helper mode for Windows.
//!
//! When KUVPN elevates itself to start a VPN tunnel it passes `--vpn-helper`
//! as the first argument so the elevated copy acts as a thin lifecycle manager
//! rather than a full GUI/CLI session.  This keeps the UAC prompt count to one
//! per connection: the helper starts OpenConnect as its own child (inheriting
//! the elevated token) and then waits for a stop-signal file to be created by
//! the non-elevated parent.  When the file appears the helper kills the child
//! and exits cleanly.
//!
//! Argument layout (positional, after `--vpn-helper`):
//!   1. oc-path    — path to the openconnect executable
//!   2. url        — VPN gateway URL
//!   3. dsid       — DSID cookie value (without the "DSID=" prefix)
//!   4. parent-pid — PID of the non-elevated parent; helper exits when it dies
//!   5. (optional) `--full-tunnel` — inject 0/1 + 128/1 routes after connect
//!
//! The stop-signal file path is NOT passed as an argument.  Both the parent and
//! the helper derive it independently as `%TEMP%\kuvpn-stop-<parent-pid>.signal`.
//! This avoids the runas crate's backslash-doubling bug, which corrupts any path
//! containing a space (e.g. `C:\Users\John Doe\...`).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// If the process was invoked with `--vpn-helper` as its first argument, runs
/// the helper and returns the exit code.  Returns `None` for normal startup.
///
/// Call this **before** any argument parser or GUI initialisation so the helper
/// can run silently without touching GUI or CLI machinery.
///
#[cfg(windows)]
pub fn run_vpn_helper_if_requested() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) != Some("--vpn-helper") {
        return None;
    }

    let oc_path = args.get(2)?;
    let url = args.get(3)?;
    let dsid = args.get(4)?;
    let parent_pid: u32 = args.get(5)?.parse().ok()?;
    let full_tunnel = args.get(6).map(|s| s == "--full-tunnel").unwrap_or(false);

    // Derive the stop-signal file path from the parent PID — same formula the
    // parent uses.  The path is never passed as an argument because the runas
    // crate doubles backslashes inside quoted args, which corrupts any path
    // whose temp-dir component contains a space (e.g. "C:\Users\John Doe\...").
    let stop_file_buf = std::env::temp_dir()
        .join(format!("kuvpn-stop-{}.signal", parent_pid));
    let stop_file = stop_file_buf.as_path();

    Some(run_helper(stop_file, oc_path, url, dsid, parent_pid, full_tunnel))
}

#[cfg(windows)]
fn run_helper(stop_file: &Path, oc_path: &str, url: &str, dsid: &str, parent_pid: u32, full_tunnel: bool) -> i32 {
    use std::os::windows::process::CommandExt;

    // Start openconnect directly — we're already elevated, so it inherits our
    // token without a second UAC prompt.  CREATE_NO_WINDOW suppresses the console.
    let mut child = match std::process::Command::new(oc_path)
        .creation_flags(CREATE_NO_WINDOW)
        .arg("--protocol")
        .arg("nc")
        .arg("-C")
        .arg(format!("DSID={}", dsid))
        .arg(url)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("vpn-helper: failed to start openconnect: {}", e);
            return 1;
        }
    };

    // When full tunnel is requested, wait for the TAP adapter to come up then
    // inject 0.0.0.0/1 and 128.0.0.0/1 routes.  These are more-specific than
    // the system default route and are automatically removed when the TAP
    // adapter goes down.
    if full_tunnel {
        if wait_for_tap_interface(30) {
            inject_full_tunnel_routes();
        } else {
            eprintln!(
                "vpn-helper: timed out waiting for TAP interface; full tunnel routes not applied"
            );
        }
    }

    // Spawn a thread that blocks on the parent process handle.  When the parent
    // exits for any reason (clean exit, crash, kill), WaitForSingleObject returns
    // and we set the flag so the loop below cleans up without a second UAC prompt.
    let parent_died = Arc::new(AtomicBool::new(false));
    let parent_died_clone = Arc::clone(&parent_died);
    std::thread::spawn(move || {
        wait_for_process_exit(parent_pid);
        parent_died_clone.store(true, Ordering::SeqCst);
    });

    // Monitor: stop when signalled by parent (stop-file), when the parent process
    // dies unexpectedly, or when openconnect exits on its own.
    loop {
        if stop_file.exists() || parent_died.load(Ordering::SeqCst) {
            // Acknowledge by removing the stop file *before* killing OC.
            // The parent watches for this deletion to know the disconnect is
            // in progress, so it doesn't fall back to an elevated kill prematurely.
            let _ = std::fs::remove_file(stop_file);
            let _ = child.kill();
            let _ = child.wait();
            return 0;
        }

        match child.try_wait() {
            Ok(Some(status)) => return if status.success() { 0 } else { 1 },
            Ok(None) => {}
            Err(e) => {
                eprintln!("vpn-helper: error waiting for child: {}", e);
                return 1;
            }
        }

        sleep(Duration::from_millis(200));
    }
}

/// Blocks until the process with the given PID exits.
/// Uses a kernel wait (zero CPU) so there is no polling overhead.
#[cfg(windows)]
fn wait_for_process_exit(pid: u32) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
    unsafe {
        match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            Ok(handle) if !handle.is_invalid() => {
                WaitForSingleObject(handle, u32::MAX); // INFINITE = 0xFFFFFFFF
                let _ = CloseHandle(handle);
            }
            _ => {} // process already gone — treat as exited
        }
    }
}

/// Polls until a TAP-Windows adapter appears with Status "Up", or until `timeout_secs` elapses.
#[cfg(windows)]
fn wait_for_tap_interface(timeout_secs: u64) -> bool {
    use std::os::windows::process::CommandExt;

    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if std::time::Instant::now() >= deadline {
            return false;
        }
        let ready = std::process::Command::new("powershell")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "-NoProfile",
                "-Command",
                "(Get-NetAdapter | Where-Object { $_.Status -eq 'Up' -and $_.InterfaceDescription -match 'TAP' }).Count",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0)
            > 0;

        if ready {
            return true;
        }
        sleep(Duration::from_millis(500));
    }
}

/// Adds 0.0.0.0/1 and 128.0.0.0/1 routes through the first connected TAP adapter.
#[cfg(windows)]
fn inject_full_tunnel_routes() {
    use std::os::windows::process::CommandExt;

    let script = "\
        $a = Get-NetAdapter | Where-Object { $_.Status -eq 'Up' -and $_.InterfaceDescription -match 'TAP' } | Select-Object -First 1; \
        if ($a) { \
            New-NetRoute -DestinationPrefix '0.0.0.0/1'   -InterfaceIndex $a.ifIndex -RouteMetric 1 -ErrorAction SilentlyContinue; \
            New-NetRoute -DestinationPrefix '128.0.0.0/1' -InterfaceIndex $a.ifIndex -RouteMetric 1 -ErrorAction SilentlyContinue \
        }";

    let _ = std::process::Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-Command", script])
        .status();
}
