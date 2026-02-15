use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use which::which;

#[cfg(windows)]
use runas::Command as AdminCommand;
#[cfg(windows)]
use sysinfo::System;

pub enum VpnProcess {
    Unix(Child),
    Windows {
        interface_name: String,
        /// Set to true when the runas background thread finishes
        /// (openconnect exited or elevation was denied).
        thread_finished: Arc<AtomicBool>,
    },
}

impl VpnProcess {
    pub fn kill(&mut self) -> anyhow::Result<()> {
        match self {
            VpnProcess::Unix(_child) => {
                #[cfg(unix)]
                {
                    use nix::sys::signal::{self, Signal};
                    use nix::unistd::Pid;
                    let pid = Pid::from_raw(_child.id() as i32);
                    let _ = signal::kill(pid, Signal::SIGINT);
                    let _ = _child.wait();
                }
                Ok(())
            }
            VpnProcess::Windows { .. } => {
                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    use std::process::Command as StdCommand;
                    const CREATE_NO_WINDOW: u32 = 0x08000000;

                    log::info!("Requesting Admin elevation to stop OpenConnect...");

                    // 1. Try killing openconnect.exe by name using Admin elevation
                    let mut cmd = AdminCommand::new("taskkill");
                    cmd.show(false);
                    cmd.args(&["/F", "/IM", "openconnect.exe", "/T"]);
                    let _ = cmd.status();

                    // Check if process was killed successfully
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if !is_openconnect_running() {
                        log::info!("OpenConnect terminated successfully");
                        return Ok(());
                    }

                    // 2. Try killing openconnect-gui.exe if it exists
                    let mut cmd_gui = AdminCommand::new("taskkill");
                    cmd_gui.show(false);
                    cmd_gui.args(&["/F", "/IM", "openconnect-gui.exe", "/T"]);
                    let _ = cmd_gui.status();

                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if !is_openconnect_running() {
                        log::info!("OpenConnect GUI terminated successfully");
                        return Ok(());
                    }

                    // 3. Try killing by PID specifically if found
                    if let Some(pid) = get_openconnect_pid() {
                        let mut cmd_pid = AdminCommand::new("taskkill");
                        cmd_pid.show(false);
                        cmd_pid.args(&["/F", "/PID", &pid.to_string(), "/T"]);
                        let _ = cmd_pid.status();

                        std::thread::sleep(std::time::Duration::from_millis(500));
                        if !is_openconnect_running() {
                            log::info!("OpenConnect terminated by PID");
                            return Ok(());
                        }
                    }

                    // 4. Last resort: non-elevated taskkill
                    log::warn!("Elevated termination failed, trying non-elevated fallback...");
                    let _ = StdCommand::new("taskkill")
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(["/F", "/IM", "openconnect.exe"])
                        .status();
                }
                Ok(())
            }
        }
    }

    /// Checks if the spawned process (sudo/doas/pkexec) is still alive.
    /// This is different from is_openconnect_running() - it checks the actual
    /// child process we spawned, not searching by process name.
    pub fn is_process_alive(&mut self) -> bool {
        match self {
            VpnProcess::Unix(_child) => {
                #[cfg(unix)]
                {
                    // try_wait returns Ok(None) if the process is still running
                    match _child.try_wait() {
                        Ok(None) => true, // still running
                        _ => false,       // exited or error
                    }
                }
                #[cfg(not(unix))]
                false
            }
            VpnProcess::Windows {
                ref thread_finished,
                ..
            } => {
                // If the runas thread is still running, the process is alive
                // (UAC prompt showing, or openconnect actively running).
                // Only check process list after the thread has finished.
                if !thread_finished.load(Ordering::SeqCst) {
                    return true;
                }
                is_openconnect_running()
            }
        }
    }

    pub fn wait(&mut self) -> anyhow::Result<()> {
        match self {
            VpnProcess::Unix(child) => {
                child.wait()?;
                Ok(())
            }
            VpnProcess::Windows {
                ref thread_finished,
                ..
            } => {
                // Wait until the runas thread finishes (openconnect exits)
                // with a timeout to prevent hanging the session thread.
                let start = std::time::Instant::now();
                while !thread_finished.load(Ordering::SeqCst) {
                    if start.elapsed() > std::time::Duration::from_secs(5) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                Ok(())
            }
        }
    }
}

/// Attempts to locate the `openconnect` executable.
pub fn locate_openconnect(user_path: &str) -> Option<PathBuf> {
    // Step 1: Check if `user_path` directly points to an existing file.
    let candidate = Path::new(user_path);
    if candidate.exists() && candidate.is_file() {
        return Some(candidate.to_path_buf());
    }

    // Step 2: Attempt to locate `openconnect` using the system's PATH.
    if let Ok(found) = which(user_path) {
        return Some(found);
    }

    // Step 3: Fallback to searching in common sbin directories.
    #[cfg(unix)]
    {
        let fallback_dirs = ["/sbin", "/usr/sbin", "/usr/local/sbin"];
        for dir in &fallback_dirs {
            let path_in_dir = Path::new(dir).join("openconnect");
            if path_in_dir.exists() && path_in_dir.is_file() {
                return Some(path_in_dir);
            }
        }
    }

    #[cfg(windows)]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let bundled_path = parent.join("openconnect").join("openconnect.exe");
                if bundled_path.exists() && bundled_path.is_file() {
                    return Some(bundled_path);
                }
            }
        }

        let common_paths = [
            "C:\\Program Files\\OpenConnect\\openconnect.exe",
            "C:\\Program Files (x86)\\OpenConnect\\openconnect.exe",
        ];
        for path in &common_paths {
            let p = Path::new(path);
            if p.exists() && p.is_file() {
                return Some(p.to_path_buf());
            }
        }
    }

    None
}

/// Searches for an available askpass program on the system.
/// Returns the path to the askpass program if found.
#[cfg(unix)]
pub fn find_askpass() -> Option<PathBuf> {
    // Check if SUDO_ASKPASS is already set
    if let Ok(askpass) = std::env::var("SUDO_ASKPASS") {
        let p = Path::new(&askpass);
        if p.exists() && p.is_file() {
            return Some(p.to_path_buf());
        }
    }

    // Search for common askpass programs
    let askpass_programs = [
        "ssh-askpass",
        "ksshaskpass",
        "lxqt-openssh-askpass",
        "x11-ssh-askpass",
        "gnome-ssh-askpass",
    ];
    for prog in &askpass_programs {
        if let Ok(path) = which(prog) {
            return Some(path);
        }
    }

    None
}

/// Determines which escalation tool will be used (for checking if password prompt is needed).
#[cfg(unix)]
pub fn resolve_escalation_tool(run_command: &Option<String>) -> Option<String> {
    let mut default_tools = vec!["doas", "sudo", "pkexec"];

    if let Some(custom_command) = run_command {
        if which(custom_command).is_ok() {
            default_tools.insert(0, custom_command.as_str());
        }
    }

    default_tools
        .iter()
        .find_map(|&tool| which(tool).ok().map(|_| tool.to_string()))
}

/// Returns true if the given escalation tool needs a password piped via stdin
/// (i.e., it's sudo or doas, not pkexec which has its own GUI prompt).
#[cfg(unix)]
pub fn needs_password_prompt(tool: &str) -> bool {
    let base = Path::new(tool)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(tool);
    matches!(base, "sudo" | "doas")
}

/// Executes the `openconnect` command.
pub fn execute_openconnect(
    cookie_value: String,
    url: String,
    _run_command: &Option<String>,
    openconnect_path: &Path,
    _stdout: Stdio,
    _stderr: Stdio,
    interface_name: &str,
    _sudo_password: Option<String>,
) -> anyhow::Result<VpnProcess> {
    #[cfg(unix)]
    {
        let mut default_tools = vec!["doas", "sudo", "pkexec"];

        if let Some(custom_command) = _run_command {
            if which(custom_command).is_ok() {
                default_tools.insert(0, custom_command.as_str());
            }
        }

        let command_to_run = default_tools
            .iter()
            .find_map(|&tool| which(tool).ok().map(|_| tool))
            .ok_or(anyhow::anyhow!(
                "No available tool for running openconnect (sudo/doas/pkexec not found)"
            ))?;

        let tool_base = Path::new(command_to_run)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(command_to_run);

        let askpass = find_askpass();
        let use_askpass = askpass.is_some() && needs_password_prompt(tool_base);
        let use_stdin_password =
            !use_askpass && _sudo_password.is_some() && needs_password_prompt(tool_base);

        let mut cmd = Command::new(command_to_run);

        // Configure password feeding method
        if use_askpass {
            let askpass_path = askpass.unwrap();
            log::info!("Using askpass program: {:?}", askpass_path);
            cmd.env("SUDO_ASKPASS", &askpass_path);
            if tool_base == "sudo" {
                cmd.arg("-A");
            }
        } else if use_stdin_password {
            log::info!("Piping password via stdin to {}", tool_base);
            if tool_base == "sudo" {
                cmd.arg("-S");
            }
            cmd.stdin(Stdio::piped());
        }

        cmd.arg(openconnect_path)
            .arg("--protocol")
            .arg("nc")
            .arg("--interface")
            .arg(interface_name)
            .arg("-C")
            .arg(format!("DSID={}", cookie_value))
            .arg(url)
            .stdout(_stdout)
            .stderr(_stderr);

        let mut child = cmd.spawn()?;

        // Pipe password to stdin if needed
        if use_stdin_password {
            if let Some(ref password) = _sudo_password {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = writeln!(stdin, "{}", password);
                    // Drop stdin to close it so the process doesn't hang waiting for more input
                    drop(stdin);
                }
            }
        }

        return Ok(VpnProcess::Unix(child));
    }

    #[cfg(windows)]
    {
        log::info!("Requesting Admin elevation for OpenConnect...");

        let interface_name_owned = interface_name.to_string();
        let mut cmd = AdminCommand::new(openconnect_path.to_str().unwrap());
        cmd.show(false);
        cmd.arg("--protocol")
            .arg("nc")
            .arg("-C")
            .arg(format!("DSID={}", cookie_value));

        cmd.arg(url);

        // runas 1.2.0 might only have status() which blocks.
        // We run it in a thread so we don't block the caller.
        // The thread_finished flag lets the watchdog know when the process exits.
        let thread_finished = Arc::new(AtomicBool::new(false));
        let finished_clone = Arc::clone(&thread_finished);

        std::thread::spawn(move || {
            match cmd.status() {
                Ok(status) => {
                    if !status.success() {
                        log::error!("OpenConnect process exited with failure.");
                    }
                }
                Err(e) => {
                    log::error!("Failed to run elevated OpenConnect: {}", e);
                }
            }
            finished_clone.store(true, Ordering::SeqCst);
        });

        Ok(VpnProcess::Windows {
            interface_name: interface_name_owned,
            thread_finished,
        })
    }
}

/// Checks if an openconnect process is currently running
pub fn is_openconnect_running() -> bool {
    #[cfg(windows)]
    {
        let mut sys = System::new_all();
        sys.refresh_all();
        for process in sys.processes().values() {
            let name = process.name().to_string_lossy().to_lowercase();
            if name.contains("openconnect") {
                return true;
            }
        }
        false
    }
    #[cfg(unix)]
    {
        let candidate = which("pgrep");
        if let Ok(pgrep) = candidate {
            let status = Command::new(pgrep)
                .arg("openconnect")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            return status.map(|s| s.success()).unwrap_or(false);
        }
        false
    }
}

/// Checks if the named VPN TUN/TAP interface is up and active.
/// More reliable than process-name detection because it verifies
/// the tunnel itself exists, not just that a process is running.
pub fn is_vpn_interface_up(interface_name: &str) -> bool {
    #[cfg(unix)]
    {
        // Check /sys/class/net/<interface_name> existence
        let sys_path = format!("/sys/class/net/{}", interface_name);
        let path = std::path::Path::new(&sys_path);
        if !path.exists() {
            return false;
        }
        // TUN devices report "unknown" when active, "down" when inactive
        let operstate_path = format!("{}/operstate", sys_path);
        if let Ok(state) = std::fs::read_to_string(&operstate_path) {
            return state.trim() != "down";
        }
        true
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::{Command as StdCommand, Stdio as StdStdio};
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = StdCommand::new("netsh")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["interface", "show", "interface"])
            .stdout(StdStdio::piped())
            .stderr(StdStdio::null())
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.contains(interface_name) && line.contains("Connected") {
                        return true;
                    }
                }
                false
            }
            Err(_) => false,
        }
    }
}

/// Gets the PID of the running openconnect process
pub fn get_openconnect_pid() -> Option<u32> {
    #[cfg(windows)]
    {
        let mut sys = System::new_all();
        sys.refresh_all();
        for (pid, process) in sys.processes() {
            let name = process.name().to_string_lossy().to_lowercase();
            if name.contains("openconnect") {
                return Some(pid.as_u32());
            }
        }
        None
    }
    #[cfg(unix)]
    {
        let candidate = which("pgrep");
        if let Ok(pgrep) = candidate {
            let output = Command::new(pgrep).arg("openconnect").output().ok()?;
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = s.lines().next() {
                    return line.parse().ok();
                }
            }
        }
        None
    }
}

/// Gracefully terminates a process by its PID.
pub fn kill_process(pid: u32) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;
        let pid = Pid::from_raw(pid as i32);
        let _ = signal::kill(pid, Signal::SIGINT);
    }
    #[cfg(windows)]
    {
        let mut cmd = AdminCommand::new("taskkill");
        cmd.show(false);
        cmd.arg("/F")
            .arg("/T")
            .arg("/PID")
            .arg(pid.to_string());
        cmd.status()?;
    }
    Ok(())
}
