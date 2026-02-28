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
//!   1. stop-file  — path that the parent creates to request a disconnect
//!   2. oc-path    — path to the openconnect executable
//!   3. url        — VPN gateway URL
//!   4. dsid       — DSID cookie value (without the "DSID=" prefix)

use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

/// If the process was invoked with `--vpn-helper` as its first argument, runs
/// the helper and returns the exit code.  Returns `None` for normal startup.
///
/// Call this **before** any argument parser or GUI initialisation so the helper
/// can run silently without touching GUI or CLI machinery.
#[cfg(windows)]
pub fn run_vpn_helper_if_requested() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) != Some("--vpn-helper") {
        return None;
    }

    let stop_file = Path::new(args.get(2)?);
    let oc_path = args.get(3)?;
    let url = args.get(4)?;
    let dsid = args.get(5)?;

    Some(run_helper(stop_file, oc_path, url, dsid))
}

#[cfg(windows)]
fn run_helper(stop_file: &Path, oc_path: &str, url: &str, dsid: &str) -> i32 {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

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

    // Monitor: stop when signalled by parent or when openconnect exits on its own.
    loop {
        if stop_file.exists() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(stop_file);
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
