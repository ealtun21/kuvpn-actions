use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use which::which;

#[cfg(windows)]
use runas::Command as AdminCommand;
#[cfg(windows)]
use sysinfo::System;

/// Attempts to locate the `openconnect` executable.
///
/// The search is performed in three steps:
/// 1. Check if the provided `user_path` directly points to an existing file.
/// 2. Use the system's PATH (via the `which` crate) to locate `openconnect`.
/// 3. Fallback: Check common sbin directories (e.g., `/sbin`, `/usr/sbin`, `/usr/local/sbin`).
///
/// # Arguments
///
/// * `user_path` - A string slice that holds the user-provided path to `openconnect`.
///
/// # Returns
///
/// * `Some(PathBuf)` if the executable is found.
/// * `None` if the executable cannot be located.
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

    // Return None if `openconnect` could not be found.
    None
}

/// Executes the `openconnect` command with elevated privileges using an available escalation tool.
///
/// The function supports a custom run command if provided; otherwise, it defaults to a list of
/// common escalation tools (`doas`, `sudo`, `pkexec`). It constructs the appropriate command-line
/// arguments and executes `openconnect` with the DSID and VPN URL.
///
/// # Arguments
///
/// * `cookie_value` - The DSID cookie value to be passed to `openconnect`.
/// * `url` - The VPN URL to connect to.
/// * `run_command` - An optional custom command for elevated privilege execution.
/// * `openconnect_path` - The resolved path to the `openconnect` executable.
///
/// # Returns
///
/// A `Result` indicating success or failure of the command execution.
pub fn execute_openconnect(
    cookie_value: String,
    url: String,
    _run_command: &Option<String>,
    openconnect_path: &Path,
    _stdout: Stdio,
    _stderr: Stdio,
) -> anyhow::Result<Child> {
    #[cfg(unix)]
    {
        // Default list of privilege escalation tools.
        let mut default_tools = vec!["doas", "sudo", "pkexec"];

        // If a custom run command is provided, check its availability and prioritize it.
        if let Some(custom_command) = _run_command {
            log::info!("Custom run command provided: {}", custom_command);
            if which(custom_command).is_ok() {
                log::info!("Custom command found: {}", custom_command);
                default_tools.insert(0, custom_command.as_str());
            } else {
                log::info!(
                    "Custom command '{}' not found, falling back to default tools.",
                    custom_command
                );
            }
        } else {
            log::info!("No custom run command provided, defaulting to built-in tools.");
        }

        // Identify the first available escalation tool from the list.
        log::info!("Checking for available tools/commands: {:?}", default_tools);
        let command_to_run = default_tools
            .iter()
            .find_map(|&tool| which(tool).ok().map(|_| tool))
            .ok_or(anyhow::anyhow!(
                "No available tool for running openconnect (sudo/doas/pkexec not found)"
            ))?;

        log::info!(
            "Running openconnect using {} for elevated privileges or execution",
            command_to_run
        );

        return Command::new(command_to_run)
            .arg(openconnect_path)
            .arg("--protocol")
            .arg("nc")
            .arg("-C")
            .arg(format!("DSID={}", cookie_value))
            .arg(url)
            .stdout(_stdout)
            .stderr(_stderr)
            .spawn()
            .map_err(anyhow::Error::from);
    }

    #[cfg(windows)]
    {
        log::info!("Requesting Admin elevation for OpenConnect...");

        let mut cmd = AdminCommand::new(openconnect_path.to_str().unwrap());
        cmd.arg("--protocol")
            .arg("nc")
            .arg("-C")
            .arg(format!("DSID={}", cookie_value));

        cmd.arg(url);

        // runas uses .status() which returns a Result<ExitStatus, io::Error>
        // This will block until the UAC prompt is accepted or rejected.
        let status = cmd
            .status()
            .map_err(|e| anyhow::anyhow!("UAC Elevation failed or was denied: {}", e))?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "OpenConnect started but returned an error exit code."
            ));
        }

        // We return a dummy child process because runas doesn't provide a handle.
        // On Windows, your GUI will track the connection state by checking
        // if 'openconnect.exe' is in the process list.
        Ok(Command::new("cmd").arg("/c").arg("exit 0").spawn()?)
    }
}

/// Checks if an openconnect process is currently running (useful for Windows)
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
        // On Unix we usually have the Child handle, but for completeness:
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
            let output = Command::new(pgrep)
                .arg("openconnect")
                .output()
                .ok()?;
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
        signal::kill(pid, Signal::SIGINT)?;
    }
    #[cfg(windows)]
    {
        // Must use elevated taskkill to kill an elevated openconnect
        AdminCommand::new("taskkill")
            .arg("/F")
            .arg("/PID")
            .arg(pid.to_string())
            .status()?;
    }
    Ok(())
}

/// Gracefully terminates the `openconnect` process.
///
/// On Unix systems, it sends a SIGINT signal to allow `openconnect` to clean up
/// network interfaces and routes. On other systems, it falls back to standard kill.
pub fn kill_child(_child: &mut Child) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;
        let pid = Pid::from_raw(_child.id() as i32);
        signal::kill(pid, Signal::SIGINT)?;
    }
    #[cfg(windows)]
    {
        // On Windows, if we don't have the PID of the elevated process,
        // we kill by image name to ensure the VPN closes.
        AdminCommand::new("taskkill")
            .arg("/F")
            .arg("/IM")
            .arg("openconnect.exe")
            .arg("/T")
            .status()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_kill_child() {
        let mut child = Command::new("sleep")
            .arg("10")
            .spawn()
            .expect("failed to execute sleep");

        let res = kill_child(&mut child);
        assert!(res.is_ok());

        let _ = child.wait();
    }
}