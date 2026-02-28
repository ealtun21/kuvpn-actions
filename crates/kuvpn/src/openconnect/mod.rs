//! OpenConnect process management.
//!
//! Platform-specific code lives in [`unix`] and [`windows`] submodules.
//! The public API of this module provides a uniform interface regardless of platform.

use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use which::which;

#[cfg(unix)]
pub(crate) mod unix;

#[cfg(windows)]
pub(crate) mod windows;

// ── Re-export platform functions ──────────────────────────────────────────────

#[cfg(unix)]
pub use unix::{
    find_askpass, get_openconnect_pid, is_openconnect_running, is_vpn_interface_up, kill_process,
    list_available_escalation_tools, needs_password_prompt, resolve_escalation_tool,
    tool_requires_password, verify_escalation_password,
};

#[cfg(windows)]
pub use windows::{get_openconnect_pid, is_openconnect_running, is_vpn_interface_up, kill_process};

// ── VpnProcess ────────────────────────────────────────────────────────────────

/// Handle to a running OpenConnect process.
///
/// On Unix this wraps the child escalation process (sudo/pkexec).
/// On Windows it holds the flag that the background runas thread sets when done.
pub enum VpnProcess {
    Unix(Child),
    Windows {
        /// Set to `true` when the runas background thread finishes.
        thread_finished: Arc<AtomicBool>,
        /// Creating this file signals the elevated helper to stop OpenConnect.
        stop_file: std::path::PathBuf,
    },
}

impl VpnProcess {
    pub fn kill(&mut self) -> anyhow::Result<()> {
        match self {
            #[cfg(unix)]
            VpnProcess::Unix(child) => unix::kill_vpn_process(child),

            #[cfg(windows)]
            VpnProcess::Windows { stop_file, .. } => windows::kill_vpn_process(stop_file),

            #[allow(unreachable_patterns)]
            _ => Ok(()),
        }
    }

    /// Returns `true` if the underlying process (or its background thread) is still alive.
    pub fn is_process_alive(&mut self) -> bool {
        match self {
            #[cfg(unix)]
            VpnProcess::Unix(child) => unix::vpn_process_alive(child),

            #[cfg(windows)]
            VpnProcess::Windows {
                thread_finished, ..
            } => windows::vpn_process_alive(thread_finished),

            #[allow(unreachable_patterns)]
            _ => false,
        }
    }

    /// Waits for the process to finish (with a 5-second timeout on Windows).
    pub fn wait(&mut self) -> anyhow::Result<()> {
        match self {
            VpnProcess::Unix(child) => {
                child.wait()?;
                Ok(())
            }
            VpnProcess::Windows {
                thread_finished, ..
            } => {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while !thread_finished.load(Ordering::SeqCst) {
                    if std::time::Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                Ok(())
            }
        }
    }
}

// ── locate_openconnect ────────────────────────────────────────────────────────

/// Attempts to locate the `openconnect` executable.
///
/// Resolution order:
/// 1. `user_path` as a literal file path.
/// 2. `user_path` looked up via `PATH`.
/// 3. Platform-specific fallback directories.
pub fn locate_openconnect(user_path: &str) -> Option<PathBuf> {
    let candidate = Path::new(user_path);
    if candidate.is_file() {
        return Some(candidate.to_path_buf());
    }

    if let Ok(found) = which(user_path) {
        return Some(found);
    }

    platform_fallback(user_path)
}

#[cfg(unix)]
fn platform_fallback(_user_path: &str) -> Option<PathBuf> {
    let dirs = [
        "/sbin",
        "/usr/sbin",
        "/usr/local/sbin",
        "/usr/local/bin",
        "/opt/homebrew/bin",
    ];
    dirs.iter()
        .map(|dir| Path::new(dir).join("openconnect"))
        .find(|p| p.is_file())
}

#[cfg(windows)]
fn platform_fallback(_user_path: &str) -> Option<PathBuf> {
    // Check next to our own executable (bundled distribution).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let bundled = parent.join("openconnect").join("openconnect.exe");
            if bundled.is_file() {
                return Some(bundled);
            }
        }
    }

    let common = [
        "C:\\Program Files\\OpenConnect\\openconnect.exe",
        "C:\\Program Files (x86)\\OpenConnect\\openconnect.exe",
    ];
    common
        .iter()
        .map(Path::new)
        .find(|p| p.is_file())
        .map(|p| p.to_path_buf())
}

#[cfg(not(any(unix, windows)))]
fn platform_fallback(_user_path: &str) -> Option<PathBuf> {
    None
}

// ── get_vpn_interface_name ────────────────────────────────────────────────────

/// Returns the name of the currently active VPN interface, or `None` if not determinable.
pub fn get_vpn_interface_name(configured_name: &str) -> Option<String> {
    #[cfg(unix)]
    return unix::vpn_interface_name(configured_name);

    #[cfg(windows)]
    {
        let _ = configured_name;
        None
    }
}

// ── OpenConnectRunner ─────────────────────────────────────────────────────────

/// Encapsulates static OpenConnect configuration (binary, interface, escalation tool).
/// Call [`execute`](Self::execute) to start the VPN tunnel.
pub struct OpenConnectRunner {
    pub path: PathBuf,
    pub interface_name: String,
    pub escalation_tool: Option<String>,
}

impl OpenConnectRunner {
    /// Locates the `openconnect` binary and returns a configured runner,
    /// or `None` if the binary cannot be found.
    pub fn locate(
        openconnect_path: &str,
        interface_name: String,
        escalation_tool: Option<String>,
    ) -> Option<Self> {
        locate_openconnect(openconnect_path).map(|path| Self {
            path,
            interface_name,
            escalation_tool,
        })
    }

    /// Starts an OpenConnect tunnel, returning the spawned [`VpnProcess`].
    pub fn execute(
        &self,
        cookie_value: String,
        url: String,
        stdout: Stdio,
        stderr: Stdio,
        sudo_password: Option<String>,
    ) -> anyhow::Result<VpnProcess> {
        execute_openconnect(
            cookie_value,
            url,
            &self.escalation_tool,
            &self.path,
            stdout,
            stderr,
            &self.interface_name,
            sudo_password,
        )
    }
}

// ── execute_openconnect ───────────────────────────────────────────────────────

/// Starts an OpenConnect tunnel. Prefer [`OpenConnectRunner::execute`] over calling
/// this directly.
#[allow(clippy::too_many_arguments)]
pub fn execute_openconnect(
    cookie_value: String,
    url: String,
    run_command: &Option<String>,
    openconnect_path: &Path,
    stdout: Stdio,
    stderr: Stdio,
    interface_name: &str,
    sudo_password: Option<String>,
) -> anyhow::Result<VpnProcess> {
    #[cfg(unix)]
    return unix::execute(
        cookie_value,
        url,
        run_command,
        openconnect_path,
        stdout,
        stderr,
        interface_name,
        sudo_password,
    );

    #[cfg(windows)]
    {
        let _ = (run_command, stdout, stderr, interface_name, sudo_password);
        return windows::execute(cookie_value, url, openconnect_path);
    }
}
