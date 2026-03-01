# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

KUVPN is a Rust VPN client for Koc University that automates browser-based authentication (Microsoft Azure AD with MFA) to retrieve a DSID cookie, then launches OpenConnect with that cookie. It has two frontends (CLI and GUI) sharing a core library.

## Build Commands

```bash
# Build all workspace crates (Linux)
cargo build

# Build specific crate
cargo build -p kuvpn-cli
cargo build -p kuvpn-gui

# Release build (uses LTO, size optimization, symbol stripping)
cargo build --release -p kuvpn-cli

# Cross-compile for Linux musl (used in CI)
cargo build --target x86_64-unknown-linux-musl --release -p kuvpn-cli

# Windows cross-compile (uses Podman container with mingw)
./scripts/build_windows.sh

# Type-check all crates for Windows (no container needed)
./scripts/check_windows.sh

# Platform-specific packaging scripts
scripts/build_appimage.sh        # Linux AppImage (x86_64 or aarch64, auto-detected)
scripts/build_macos.sh           # macOS .app/.dmg/.pkg (incomplete)
./scripts/build_windows.sh       # Windows cross-compile (MUST be run on Linux)
```

**Important**: Always build both Linux (`cargo build`) AND Windows (`./scripts/build_windows.sh`) to verify changes, as the codebase has significant `#[cfg(unix)]` / `#[cfg(windows)]` branching. rust-analyzer only checks one target at a time. Note that `./scripts/build_windows.sh` is a cross-compilation script designed to run on Linux (using a Podman/Docker container). **Claude (the agent) is capable of running this script and should do so to verify Windows builds.**

## Latest Status & Fixes (Feb 28, 2026)

### Fixed
- **Interface Display:** "Interface: kuvpn0" is now hidden on Windows.
- **Tray Debounce:** Basic tray click show/hide is now more stable.
- **Disconnection Detection:** The app now correctly detects and reports when OpenConnect fails to stop on Windows.
- **Tray Icons:** Integrated custom SVG tray icons that automatically change based on connection status:
  - Normal (idle/ready): Shows KU shield logo
  - Connected: Shows shield with green checkmark badge
  - Disconnected/Error: Shows shield with red X badge
  - Connecting/Disconnecting: Shows shield with orange badge (new)
- **Tray Tooltip:** Now updates dynamically to show connection state (e.g. "KUVPN — Connected").
- **Windows Tray Icon Clarity:** Icons now render at 32×32 on Windows instead of 512×512, eliminating blurriness from OS downscaling.
- **Minimize + Auto-focus:** `show_or_focus_window` now calls `minimize(id, false)` before `gain_focus`, so MFA prompts and tray-click restores correctly unminimize the window on all platforms.

No test suite exists. Verification is done by building and running the binaries manually.

## Architecture

**Workspace layout** (`Cargo.toml` at root):
- `crates/kuvpn` - Core library shared by both frontends
- `crates/kuvpn-cli` - CLI frontend (clap for arg parsing)
- `crates/kuvpn-gui` - GUI frontend (Iced framework, system tray via tray-icon, SVG icons)

### Core Library (`crates/kuvpn`)
... (skipping some lines)
### GUI Architecture (`crates/kuvpn-gui`)

Built with Iced 0.14. Key files:
- `app.rs` - Main application struct `KuVpnGui`, message handling (Iced `update`/`view` pattern)
- `types.rs` - `Message` enum, styling constants, and SVG icon data
- `config.rs` - JSON settings persistence (`gui_settings.json` in user data dir)
... (skipping some lines)

## Documentation Structure

Three-page docs live in `docs/`:

- `README.md` — landing page with install commands and links to the two doc pages
- `docs/cli.md` — CLI (`kuvpn`) documentation: flags, login modes, troubleshooting
- `docs/gui.md` — GUI (`KUVPN`) documentation: install per platform, usage, settings, troubleshooting

## Issues to be fixed

- **Windows ARM** — no free GitHub-hosted Windows ARM runner available yet. Revisit when runners become available. (BACKLOG)