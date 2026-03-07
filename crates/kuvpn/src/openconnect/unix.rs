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

// ── vpnc-script generation ────────────────────────────────────────────────────

/// A generated vpnc-script written to a temporary file.
/// The file is automatically deleted when this value is dropped.
pub struct TempScript {
    path: PathBuf,
}

impl TempScript {
    /// Returns the path to the script as a `&str`, or `None` on non-UTF-8 paths.
    pub fn path_str(&self) -> Option<&str> {
        self.path.to_str()
    }
}

impl Drop for TempScript {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Generates a vpnc-script for the given `TunnelMode` (Split or Full), writes it
/// to a temporary file, makes it executable, and returns a [`TempScript`] handle.
///
/// The script handles routing and DNS setup/teardown on both macOS and Linux,
/// fixing the macOS-specific `networksetup` failure in openconnect's built-in script.
pub fn generate_vpnc_script(
    mode: &crate::session::TunnelMode,
) -> anyhow::Result<TempScript> {
    let mode_str = match mode {
        crate::session::TunnelMode::Full => "full",
        _ => "split",
    };

    // The script is baked in at compile time; only TUNNEL_MODE is substituted.
    let script = VPNC_SCRIPT_TEMPLATE.replace("{TUNNEL_MODE}", mode_str);

    // Pick a unique temp path: /tmp/kuvpn-vpnc-<pid>.sh
    let path = std::env::temp_dir()
        .join(format!("kuvpn-vpnc-{}.sh", std::process::id()));

    std::fs::write(&path, script.as_bytes())?;

    // Make executable (rwxr-xr-x)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(TempScript { path })
}

/// Cross-platform vpnc-script template.
/// Handles routing and DNS for both macOS (scutil) and Linux (resolvectl / resolv.conf).
/// `{TUNNEL_MODE}` is substituted with `"full"` or `"split"` at runtime.
const VPNC_SCRIPT_TEMPLATE: &str = r#"#!/bin/sh
# kuvpn generated vpnc-script — do not edit manually.
# Handles routing and DNS for openconnect VPN tunnels.

TUNNEL_MODE="{TUNNEL_MODE}"
OS="$(uname -s)"

setup_interface() {
    if [ "$OS" = "Darwin" ]; then
        # Assign the VPN IP to the utun device (point-to-point, both ends = our address).
        # openconnect delegates this to the script when --script is provided.
        ifconfig "$TUNDEV" "$INTERNAL_IP4_ADDRESS" "$INTERNAL_IP4_ADDRESS" \
            mtu "${INTERNAL_IP4_MTU:-1400}" netmask 255.255.255.255 up
        # Host route makes the kernel aware that $INTERNAL_IP4_ADDRESS lives on this
        # tunnel, which is required for 'route add default $INTERNAL_IP4_ADDRESS' to work.
        route add -host "$INTERNAL_IP4_ADDRESS" -interface "$TUNDEV" 2>/dev/null || true
    else
        ip addr add "${INTERNAL_IP4_ADDRESS}/${INTERNAL_IP4_NETMASKLEN:-24}" dev "$TUNDEV" 2>/dev/null || true
        ip link set "$TUNDEV" mtu "${INTERNAL_IP4_MTU:-1400}" up
    fi
}

teardown_interface() {
    if [ "$OS" = "Darwin" ]; then
        route delete -host "$INTERNAL_IP4_ADDRESS" 2>/dev/null || true
        # openconnect closes the utun fd itself; no ifconfig down needed
    else
        ip addr del "${INTERNAL_IP4_ADDRESS}/${INTERNAL_IP4_NETMASKLEN:-24}" dev "$TUNDEV" 2>/dev/null || true
        ip link set "$TUNDEV" down 2>/dev/null || true
    fi
}

setup_routes() {
    if [ "$TUNNEL_MODE" = "full" ]; then
        if [ "$OS" = "Darwin" ]; then
            real_gw=$(route -n get default 2>/dev/null | awk '/gateway:/{print $2}')
            # Protect the VPN server itself so it stays reachable via WiFi
            [ -n "$real_gw" ] && [ -n "$VPNGATEWAY" ] && \
                route add -host "$VPNGATEWAY" "$real_gw" 2>/dev/null || true
            # Save old default gateway for teardown restoration
            GW_FILE="/tmp/kuvpn-gw-${TUNDEV}.saved"
            [ -n "$real_gw" ] && echo "$real_gw" > "$GW_FILE"
            # Replace default route with VPN tunnel (matches openconnect built-in behavior)
            route delete default 2>/dev/null || true
            route add default "$INTERNAL_IP4_ADDRESS" 2>/dev/null || true
        else
            real_gw=$(ip route show default 2>/dev/null | awk '/default/{print $3; exit}')
            [ -n "$real_gw" ] && [ -n "$VPNGATEWAY" ] && \
                ip route add "$VPNGATEWAY/32" via "$real_gw" 2>/dev/null || true
            ip route add 0.0.0.0/1   dev "$TUNDEV" 2>/dev/null || true
            ip route add 128.0.0.0/1 dev "$TUNDEV" 2>/dev/null || true
        fi
    else
        i=0
        while [ "$i" -lt "${CISCO_SPLIT_INC:-0}" ]; do
            eval "addr=\$CISCO_SPLIT_INC_${i}_ADDR"
            eval "masklen=\$CISCO_SPLIT_INC_${i}_MASKLEN"
            if [ -n "$addr" ] && [ -n "$masklen" ]; then
                if [ "$OS" = "Darwin" ]; then
                    if [ "$addr" = "0.0.0.0" ] && [ "$masklen" = "0" ]; then
                        # Server pushed 0/0 — can't 'route add' over existing default;
                        # do the same default-route replacement as full tunnel.
                        real_gw=$(route -n get default 2>/dev/null | awk '/gateway:/{print $2}')
                        [ -n "$real_gw" ] && [ -n "$VPNGATEWAY" ] && \
                            route add -host "$VPNGATEWAY" "$real_gw" 2>/dev/null || true
                        GW_FILE="/tmp/kuvpn-gw-${TUNDEV}.saved"
                        [ -n "$real_gw" ] && echo "$real_gw" > "$GW_FILE"
                        route delete default 2>/dev/null || true
                        route add default "$INTERNAL_IP4_ADDRESS" 2>/dev/null || true
                    else
                        route add -net "$addr/$masklen" "$INTERNAL_IP4_ADDRESS" 2>/dev/null || true
                    fi
                else
                    if [ "$addr" = "0.0.0.0" ] && [ "$masklen" = "0" ]; then
                        real_gw=$(ip route show default 2>/dev/null | awk '/default/{print $3; exit}')
                        [ -n "$real_gw" ] && [ -n "$VPNGATEWAY" ] && \
                            ip route add "$VPNGATEWAY/32" via "$real_gw" 2>/dev/null || true
                        ip route add 0.0.0.0/1   dev "$TUNDEV" 2>/dev/null || true
                        ip route add 128.0.0.0/1 dev "$TUNDEV" 2>/dev/null || true
                    else
                        ip route add "$addr/$masklen" dev "$TUNDEV" 2>/dev/null || true
                    fi
                fi
            fi
            i=$((i + 1))
        done
    fi
}

teardown_routes() {
    if [ "$TUNNEL_MODE" = "full" ]; then
        if [ "$OS" = "Darwin" ]; then
            # Restore original default route
            GW_FILE="/tmp/kuvpn-gw-${TUNDEV}.saved"
            route delete default 2>/dev/null || true
            if [ -f "$GW_FILE" ]; then
                OLD_GW=$(cat "$GW_FILE")
                [ -n "$OLD_GW" ] && route add default "$OLD_GW" 2>/dev/null || true
                rm -f "$GW_FILE"
            fi
            [ -n "$VPNGATEWAY" ] && route delete -host "$VPNGATEWAY" 2>/dev/null || true
        else
            ip route del 0.0.0.0/1   dev "$TUNDEV" 2>/dev/null || true
            ip route del 128.0.0.0/1 dev "$TUNDEV" 2>/dev/null || true
            [ -n "$VPNGATEWAY" ] && ip route del "$VPNGATEWAY/32" 2>/dev/null || true
        fi
    else
        # Split teardown: check if we did a default-route replacement (0/0 case)
        GW_FILE="/tmp/kuvpn-gw-${TUNDEV}.saved"
        if [ "$OS" = "Darwin" ] && [ -f "$GW_FILE" ]; then
            route delete default 2>/dev/null || true
            OLD_GW=$(cat "$GW_FILE")
            [ -n "$OLD_GW" ] && route add default "$OLD_GW" 2>/dev/null || true
            rm -f "$GW_FILE"
            [ -n "$VPNGATEWAY" ] && route delete -host "$VPNGATEWAY" 2>/dev/null || true
        else
        i=0
        while [ "$i" -lt "${CISCO_SPLIT_INC:-0}" ]; do
            eval "addr=\$CISCO_SPLIT_INC_${i}_ADDR"
            eval "masklen=\$CISCO_SPLIT_INC_${i}_MASKLEN"
            if [ -n "$addr" ] && [ -n "$masklen" ]; then
                if [ "$OS" = "Darwin" ]; then
                    route delete -net "$addr/$masklen" "$INTERNAL_IP4_ADDRESS" 2>/dev/null || true
                else
                    ip route del "$addr/$masklen" dev "$TUNDEV" 2>/dev/null || true
                fi
            fi
            i=$((i + 1))
        done
        fi
    fi
}

setup_dns() {
    [ -z "$INTERNAL_IP4_DNS" ] && return 0
    if [ "$OS" = "Darwin" ]; then
        {
            echo "open"
            echo "d.init"
            echo "d.add ServerAddresses * $INTERNAL_IP4_DNS"
            [ -n "$CISCO_DEF_DOMAIN" ] && echo "d.add SearchDomains * $CISCO_DEF_DOMAIN"
            echo "set State:/Network/Service/${TUNDEV}/DNS"
            echo "quit"
        } | scutil
    else
        if command -v resolvectl >/dev/null 2>&1; then
            # shellcheck disable=SC2086
            resolvectl dns    "$TUNDEV" $INTERNAL_IP4_DNS 2>/dev/null || true
            [ -n "$CISCO_DEF_DOMAIN" ] && \
                resolvectl domain "$TUNDEV" "$CISCO_DEF_DOMAIN" 2>/dev/null || true
        elif [ -w /etc/resolv.conf ]; then
            cp /etc/resolv.conf /etc/resolv.conf.kuvpn.bak 2>/dev/null || true
            {
                for dns in $INTERNAL_IP4_DNS; do echo "nameserver $dns"; done
                [ -n "$CISCO_DEF_DOMAIN" ] && echo "search $CISCO_DEF_DOMAIN"
                cat /etc/resolv.conf.kuvpn.bak
            } > /tmp/kuvpn-resolv.conf && mv /tmp/kuvpn-resolv.conf /etc/resolv.conf
        fi
    fi
}

teardown_dns() {
    if [ "$OS" = "Darwin" ]; then
        {
            echo "open"
            echo "remove State:/Network/Service/${TUNDEV}/DNS"
            echo "quit"
        } | scutil
    else
        if command -v resolvectl >/dev/null 2>&1; then
            resolvectl revert "$TUNDEV" 2>/dev/null || true
        elif [ -f /etc/resolv.conf.kuvpn.bak ]; then
            mv /etc/resolv.conf.kuvpn.bak /etc/resolv.conf 2>/dev/null || true
        fi
    fi
}

case "$reason" in
    pre-init)
        ;;
    connect|reconnect)
        setup_interface
        setup_routes
        setup_dns
        ;;
    disconnect)
        teardown_dns
        teardown_routes
        teardown_interface
        ;;
    attempt-reconnect)
        ;;
esac
"#;

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
    custom_script: Option<&str>,
    verbose: bool,
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

    if let Some(script) = custom_script {
        cmd.arg("--script").arg(script);
    }

    if verbose {
        cmd.arg("--verbose");
    }

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
            // Extract the IP address (second token on the line)
            let ip = line.split_whitespace().nth(1).unwrap_or("");
            // Skip CGNAT range 100.64.0.0/10 (100.64–100.127) used by Tailscale/WireGuard
            if !is_cgnat_address(ip) {
                found = current_utun.clone();
            }
        }
    }
    found
}

/// Returns `true` for addresses in the CGNAT range 100.64.0.0/10
/// (100.64.0.0 – 100.127.255.255), which is used by Tailscale and similar
/// mesh VPNs and should never be assigned by an openconnect VPN.
#[cfg(target_os = "macos")]
fn is_cgnat_address(addr: &str) -> bool {
    let mut parts = addr.split('.');
    let a: u8 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let b: u8 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    a == 100 && (64..=127).contains(&b)
}
