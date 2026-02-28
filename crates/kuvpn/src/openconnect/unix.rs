//! Unix-specific OpenConnect process management.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use which::which;

use super::VpnProcess;

// ── Private helpers ───────────────────────────────────────────────────────────

/// Returns the basename of a tool path (e.g. "/usr/bin/sudo" → "sudo").
fn tool_base_name(tool: &str) -> &str {
    Path::new(tool)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(tool)
}

/// Runs `tool` (looked up via PATH) with `args`, suppressing output.
/// Returns `true` if the command exits successfully.
fn run_command_silent(tool: &str, args: &[&str]) -> bool {
    which(tool).ok().is_some_and(|p| {
        Command::new(p)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Attempts to send SIGTERM to `pid` via sudo, sudo-rs, or pkexec (in that order).
/// Returns `true` if at least one tool succeeded.
pub(super) fn try_kill_elevated(pid: u32) -> bool {
    let pid_str = pid.to_string();
    let via_sudo = ["sudo", "sudo-rs"]
        .iter()
        .any(|&tool| run_command_silent(tool, &["-n", "kill", "-15", &pid_str]));
    via_sudo || run_command_silent("pkexec", &["kill", &pid_str])
}

// ── Platform implementations (VpnProcess delegation) ─────────────────────────

/// Terminates the child process (escalation wrapper + openconnect itself).
pub(super) fn kill_vpn_process(child: &mut Child) -> anyhow::Result<()> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    let pid = Pid::from_raw(child.id() as i32);

    // Send SIGTERM to the escalation process (sudo forwards it; pkexec does not).
    let _ = signal::kill(pid, Signal::SIGTERM);

    // Kill openconnect directly via the privilege tool (handles pkexec case and caches sudo).
    if let Some(oc_pid) = get_openconnect_pid() {
        let _ = try_kill_elevated(oc_pid);
    }

    // Wait with a 5-second timeout; fall back to SIGKILL.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = signal::kill(pid, Signal::SIGKILL);
                break;
            }
            Err(_) => break,
            _ => std::thread::sleep(std::time::Duration::from_millis(200)),
        }
    }
    let _ = child.wait();
    Ok(())
}

pub(super) fn vpn_process_alive(child: &mut Child) -> bool {
    matches!(child.try_wait(), Ok(None))
}

// ── Public platform functions ─────────────────────────────────────────────────

/// Searches for an available askpass program on the system.
pub fn find_askpass() -> Option<PathBuf> {
    if let Ok(askpass) = std::env::var("SUDO_ASKPASS") {
        let p = Path::new(&askpass);
        if p.exists() && p.is_file() {
            return Some(p.to_path_buf());
        }
    }

    let programs = [
        "ssh-askpass",
        "ksshaskpass",
        "lxqt-openssh-askpass",
        "x11-ssh-askpass",
        "gnome-ssh-askpass",
    ];
    programs.iter().find_map(|&prog| which(prog).ok())
}

/// Resolves which escalation tool will be used.
/// Checks `run_command` first, then falls back to `sudo`, `sudo-rs`, and `pkexec`.
pub fn resolve_escalation_tool(run_command: &Option<String>) -> Option<String> {
    let mut tools = vec!["sudo", "sudo-rs", "pkexec"];

    if let Some(custom) = run_command {
        if which(custom.as_str()).is_ok() {
            tools.insert(0, custom.as_str());
        }
    }

    tools
        .iter()
        .find_map(|&t| which(t).ok().map(|_| t.to_string()))
}

/// Returns the ordered list of escalation tools that are installed on this system.
/// On macOS, `pkexec` is excluded as it is unavailable there.
pub fn list_available_escalation_tools() -> Vec<&'static str> {
    let candidates: &[&'static str] = if cfg!(target_os = "macos") {
        &["sudo", "sudo-rs"]
    } else {
        &["sudo", "sudo-rs", "pkexec"]
    };
    candidates
        .iter()
        .copied()
        .filter(|&t| which(t).is_ok())
        .collect()
}

/// Returns `true` if the given tool needs a password piped via stdin
/// (i.e. it is `sudo` or `sudo-rs`, not `pkexec` which provides its own GUI prompt).
pub fn needs_password_prompt(tool: &str) -> bool {
    matches!(tool_base_name(tool), "sudo" | "sudo-rs")
}

/// Returns `true` if the tool would block for a password right now
/// (credentials are not already cached / NOPASSWD not configured).
pub fn tool_requires_password(tool: &str) -> bool {
    let base = tool_base_name(tool);
    // pkexec always uses its own GUI prompt — never needs stdin.
    matches!(base, "sudo" | "sudo-rs") && !run_command_silent(base, &["-n", "true"])
}

/// Verifies a sudo/sudo-rs password with a quick non-interactive test.
/// Returns `Some(true)` if accepted, `Some(false)` if rejected, `None` if not applicable.
pub fn verify_escalation_password(tool: &str, password: &str) -> Option<bool> {
    let base = tool_base_name(tool);
    if !matches!(base, "sudo" | "sudo-rs") {
        return None;
    }

    let tool_path = which(base).ok()?;
    let mut child = Command::new(&tool_path)
        .args(["-S", "true"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = writeln!(stdin, "{}", password);
    }

    Some(child.wait().is_ok_and(|s| s.success()))
}

/// Checks whether an openconnect process is currently running.
pub fn is_openconnect_running() -> bool {
    which("pgrep").is_ok_and(|pgrep| {
        Command::new(pgrep)
            .arg("openconnect")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Returns the PID of the running openconnect process, if any.
pub fn get_openconnect_pid() -> Option<u32> {
    let pgrep = which("pgrep").ok()?;
    let output = Command::new(pgrep).arg("openconnect").output().ok()?;
    output
        .status
        .success()
        .then(|| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()?
                .parse()
                .ok()
        })
        .flatten()
}

/// Gracefully terminates a process by PID (elevated, with SIGKILL fallback).
pub fn kill_process(pid: u32) -> anyhow::Result<()> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    if !try_kill_elevated(pid) {
        // Fallback: direct signal (may fail with EPERM for root processes).
        let nix_pid = Pid::from_raw(pid as i32);
        let _ = signal::kill(nix_pid, Signal::SIGTERM);
        std::thread::sleep(std::time::Duration::from_millis(1000));
        let _ = signal::kill(nix_pid, Signal::SIGKILL);
    }
    Ok(())
}

/// Returns `true` if the VPN interface is up and active.
pub fn is_vpn_interface_up(interface_name: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        let _ = interface_name;
        return detect_active_utun().is_some();
    }

    #[cfg(not(target_os = "macos"))]
    {
        let sys_path = format!("/sys/class/net/{}", interface_name);
        if !std::path::Path::new(&sys_path).exists() {
            return false;
        }
        let operstate = format!("{}/operstate", sys_path);
        std::fs::read_to_string(&operstate)
            .map(|s| s.trim() != "down")
            .unwrap_or(true)
    }
}

/// Returns the active VPN interface name for this platform.
pub fn vpn_interface_name(configured_name: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let _ = configured_name;
        return detect_active_utun();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let sys_path = format!("/sys/class/net/{}", configured_name);
        std::path::Path::new(&sys_path)
            .exists()
            .then(|| configured_name.to_string())
    }
}

/// Executes openconnect on Unix (via sudo/pkexec/etc).
#[allow(clippy::too_many_arguments)]
pub(super) fn execute(
    cookie_value: String,
    url: String,
    run_command: &Option<String>,
    openconnect_path: &Path,
    stdout: Stdio,
    stderr: Stdio,
    // On macOS openconnect auto-assigns a utun%d interface; the name is ignored.
    #[cfg_attr(target_os = "macos", allow(unused_variables))] interface_name: &str,
    sudo_password: Option<String>,
) -> anyhow::Result<VpnProcess> {
    let command_to_run = resolve_escalation_tool(run_command).ok_or_else(|| {
        anyhow::anyhow!(
            "No privilege escalation tool found (sudo, sudo-rs, or pkexec). \
             Please install one of them to run openconnect."
        )
    })?;

    let tool_base = tool_base_name(&command_to_run);
    let askpass = find_askpass();
    let use_askpass = askpass.is_some() && needs_password_prompt(tool_base);
    let use_stdin_password =
        !use_askpass && sudo_password.is_some() && needs_password_prompt(tool_base);

    let mut cmd = Command::new(&command_to_run);

    match (use_askpass, use_stdin_password) {
        (true, _) => {
            let askpass_path = askpass.unwrap();
            log::info!("Using askpass program: {:?}", askpass_path);
            cmd.env("SUDO_ASKPASS", &askpass_path);
            if matches!(tool_base, "sudo" | "sudo-rs") {
                cmd.arg("-A");
            }
        }
        (false, true) => {
            log::info!("Piping password via stdin to {}", tool_base);
            if matches!(tool_base, "sudo" | "sudo-rs") {
                cmd.arg("-S");
            }
            cmd.stdin(Stdio::piped());
        }
        _ => {}
    }

    cmd.arg(openconnect_path).arg("--protocol").arg("nc");

    // macOS does not support custom TUN interface names; openconnect auto-assigns utun%d.
    #[cfg(not(target_os = "macos"))]
    cmd.arg("--interface").arg(interface_name);

    cmd.arg("-C")
        .arg(format!("DSID={}", cookie_value))
        .arg(url)
        .stdout(stdout)
        .stderr(stderr);

    let mut child = cmd.spawn()?;

    if use_stdin_password {
        if let (Some(password), Some(mut stdin)) = (&sudo_password, child.stdin.take()) {
            let _ = writeln!(stdin, "{}", password);
            drop(stdin);
        }
    }

    Ok(VpnProcess::Unix(child))
}

// ── macOS-specific ────────────────────────────────────────────────────────────

/// Detects an active VPN utun interface on macOS by parsing `ifconfig` output.
/// Returns the name of a utun interface that carries an IPv4 (`inet`) address,
/// indicating an active tunnel (system-managed utuns carry only IPv6 link-local).
#[cfg(target_os = "macos")]
fn detect_active_utun() -> Option<String> {
    let output = Command::new("/sbin/ifconfig").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut current_utun: Option<String> = None;
    let mut found: Option<String> = None;

    for line in stdout.lines() {
        if !line.starts_with('\t') && !line.starts_with(' ') {
            // Interface header: "utunN: flags=..."
            current_utun = line.find(':').and_then(|pos| {
                let iface = &line[..pos];
                iface.starts_with("utun").then(|| iface.to_string())
            });
        } else if current_utun.is_some() && line.trim_start().starts_with("inet ") {
            found = current_utun.clone();
        }
    }
    found
}
