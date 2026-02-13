# TODO

## KUVPN GUI

- [x] Switch icons from Nerd Font to vector/SVG assets (cross-platform font rendering is unreliable).
- [x] Cleanup browser handling.
- [x] Add connecting/disconnecting animation to status icon.
- [ ] GUI visual improvements (backlog - get stability first).

## KUVPN LIB - REFACTORING

- [x] Create a `VpnSession` manager in the library to unify connection logic.
- [x] Implement robust connection state tracking (Connected/Disconnected/Connecting) in the library.
- [x] Remove log-based state detection.
- [x] Improve OpenConnect process management on Windows.
- [x] Ensure browser is strictly closed before starting OpenConnect.
- [x] Refactor `run_login_and_get_dsid` to be more robust and less reliant on generic retries.
- [x] Replace process-name detection with TUN interface detection (`--interface kuvpn0`).
- [x] Fix GUI/CLI openconnect connection tracking (now uses `is_vpn_interface_up()`).
- [x] Cleanup openconnect.rs and VpnProcess::Windows (now stores interface_name).
- [x] Deduplicate log level / login mode matching in GUI code.
- [x] Hide OpenConnect console window on Windows.
- [x] Improve browser robustness (handle manual closure, prevent 'session crashed' bubbles).
- [x] Fix Windows focus flicker caused by background connection checks.
- [x] Fix Windows visibility toggle spawning multiple windows.
- [x] Implement single-instance check for the application.

## BACKLOG

- macOS builds (not complete yet)
- Investigate native `--interface` support on newer Windows OpenConnect builds.
