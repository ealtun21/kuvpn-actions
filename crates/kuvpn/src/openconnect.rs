use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
#[cfg(unix)]
use std::process::Command;
use which::which;

#[cfg(windows)]
use runas::Command as AdminCommand;
#[cfg(windows)]
use sysinfo::System;

pub enum VpnProcess {
    Unix(Child),
    Windows, // On Windows we track via process list because runas is tricky
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
            VpnProcess::Windows => {
                if let Some(pid) = get_openconnect_pid() {
                    kill_process(pid)?;
                }
                Ok(())
            }
        }
    }

    pub fn wait(&mut self) -> anyhow::Result<()> {
        match self {
            VpnProcess::Unix(child) => {
                child.wait()?;
                Ok(())
            }
            VpnProcess::Windows => {
                while is_openconnect_running() {
                    std::thread::sleep(std::time::Duration::from_millis(500));
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

/// Executes the `openconnect` command.
pub fn execute_openconnect(
    cookie_value: String,
    url: String,
    _run_command: &Option<String>,
    openconnect_path: &Path,
    _stdout: Stdio,
    _stderr: Stdio,
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

        let child = Command::new(command_to_run)
            .arg(openconnect_path)
            .arg("--protocol")
            .arg("nc")
            .arg("-C")
            .arg(format!("DSID={}", cookie_value))
            .arg(url)
            .stdout(_stdout)
            .stderr(_stderr)
            .spawn()?;

        return Ok(VpnProcess::Unix(child));
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

        // runas 1.2.0 might only have status() which blocks.
        // We run it in a thread so we don't block the caller.
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
        });

        Ok(VpnProcess::Windows)
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
        let _ = signal::kill(pid, Signal::SIGINT);
    }
    #[cfg(windows)]
    {
        AdminCommand::new("taskkill")
            .arg("/F")
            .arg("/PID")
            .arg(pid.to_string())
            .status()?;
    }
    Ok(())
}