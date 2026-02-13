use log::info;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use which::which;

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
    stdout: Stdio,
    stderr: Stdio,
) -> anyhow::Result<Child> {
    #[cfg(unix)]
    {
        // Default list of privilege escalation tools.
        let mut default_tools = vec!["doas", "sudo", "pkexec"];

        // If a custom run command is provided, check its availability and prioritize it.
        if let Some(custom_command) = _run_command {
            info!("Custom run command provided: {}", custom_command);
            if which(custom_command).is_ok() {
                info!("Custom command found: {}", custom_command);
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
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map_err(anyhow::Error::from);
    }

    #[cfg(windows)]
    {
        log::info!("Running openconnect on Windows: {:?}", openconnect_path);
        // On Windows, we might need to use 'runas' to elevate if not already elevated.
        // However, Command::new doesn't easily support 'runas' for child processes with piped I/O.
        // For now, we assume the app is run as Admin or openconnect.exe has manifest for elevation.
        return Command::new(openconnect_path)
            .arg("--protocol")
            .arg("nc")
            .arg("-C")
            .arg(format!("DSID={}", cookie_value))
            .arg(url)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map_err(anyhow::Error::from);
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
        use std::process::Command;
        // Use taskkill /F /PID <pid> as a fallback on Windows
        // Ideally we'd use GenerateConsoleCtrlEvent but that's complex
        Command::new("taskkill")
            .arg("/F")
            .arg("/PID")
            .arg(pid.to_string())
            .spawn()?
            .wait()?;
    }
    Ok(())
}

/// Gracefully terminates the `openconnect` process.
///
/// On Unix systems, it sends a SIGINT signal to allow `openconnect` to clean up
/// network interfaces and routes. On other systems, it falls back to standard kill.
pub fn kill_child(child: &mut Child) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;
        let pid = Pid::from_raw(child.id() as i32);
        signal::kill(pid, Signal::SIGINT)?;
    }
    #[cfg(not(unix))]
    {
        child.kill()?;
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
