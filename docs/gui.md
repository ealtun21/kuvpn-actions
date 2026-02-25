# KUVPN — GUI Documentation

[← Back to README](../README.md) · [CLI Documentation →](cli.md)

**KUVPN** is the graphical frontend. It lives in your system tray, sends OS notifications for MFA and connection events, and automatically brings itself to focus when it needs your input.

---

## Installation

### Linux

**Recommended:**

```bash
curl -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash
```

The script will ask what to install — choose GUI. It also checks for OpenConnect and offers to install it.

<details><summary>Non-interactive (for scripting / automation)</summary>

```bash
curl -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash -s -- --gui
```

</details>

<details><summary>Manual install</summary>

Download **`KUVPN-linux-x86_64.AppImage`** (or `aarch64`) from the [Releases](https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/releases/latest) page, make it executable, and run it:

```bash
chmod +x KUVPN-linux-x86_64.AppImage && ./KUVPN-linux-x86_64.AppImage
```

</details>

### macOS

**Recommended:**

```bash
curl -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash
```

The script will ask what to install — choose GUI. It mounts the DMG, copies to Applications, and removes the quarantine flag automatically.

<details><summary>Non-interactive (for scripting / automation)</summary>

```bash
curl -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash -s -- --gui
```

</details>

<details><summary>Manual install</summary>

Download **`KUVPN-macOS-x86_64.dmg`** (Intel) or **`KUVPN-macOS-aarch64.dmg`** (Apple Silicon) from the [Releases](https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/releases/latest) page.

Open the DMG and drag **KUVPN.app** to your Applications folder, then run:

```bash
sudo xattr -r -d com.apple.quarantine /Applications/KUVPN.app
```

This removes the macOS quarantine flag that would otherwise block the app from opening (since it is not notarized through the App Store).

</details>

### Windows

Download and run **`KUVPN-Setup-windows-x86_64.exe`** from the [Releases](https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/releases/latest) page. The installer bundles OpenConnect and Wintun — no extra setup required.

---

## First Launch

When KUVPN opens you will see three tabs: **Connection**, **Console**, and **Settings**.

Before connecting, go to **Settings** and verify:

- **OpenConnect path** — the app auto-detects it on startup and shows a green checkmark if found. If it is red, install OpenConnect or enter the full path manually.
- **Email** — entering your university email allows the auto-login to skip the email field.
- **Login mode** — see [Login Modes](#login-modes) below.
- **Escalation tool** (Linux/macOS) — the tool used to run OpenConnect as root (`sudo` or `pkexec`). The app picks one automatically but you can override it.

---

## Connecting

1. Switch to the **Connection** tab.
2. Click **Connect**.
3. The status changes to *Connecting* and the Console tab will show live log output.
4. If MFA is required, KUVPN sends an OS notification and brings the window to the front so you can respond.
5. Once connected, the status shows *Connected* and the tray icon turns green.

To disconnect, click **Disconnect** or use the tray menu.

---

## Login Modes

The slider in Settings controls how much KUVPN automates the login:

| Mode | Description |
|------|-------------|
| **Full Auto** | Browser runs headlessly. KUVPN fills in all fields automatically. Best for everyday use once your session is established. |
| **Visual Auto** | Browser window is visible but automation still runs. Useful for debugging or when you want to watch what's happening. |
| **Manual** | Browser window opens and you complete the login yourself. KUVPN waits for the DSID cookie, then starts OpenConnect. Use this the first time or when auto-login fails. |

After a successful manual login the session is saved, so Full Auto will work on future connects.

---

## System Tray

KUVPN minimises to the system tray when you close the window (if **Close to tray** is enabled in Settings).

The tray icon reflects the current connection state:

| Icon | State |
|------|-------|
| Shield (normal) | Idle / ready |
| Shield + green checkmark | Connected |
| Shield + red X | Disconnected / error |

Right-clicking the tray icon gives you a menu to show/hide the window, connect, disconnect, or quit.

---

## Notifications

KUVPN sends OS notifications for:

- **MFA required** — shows the push code directly in the notification so you can approve it from your phone without switching back to the app. If a code needs to be typed, the app comes to the foreground automatically.
- **Action required** — when the VPN process needs input (e.g., a password prompt), you are notified and the window is brought to focus.
- **Connected** — confirmation when the tunnel is up.
- **Connection error** — shown when something goes wrong, with a brief description.

---

## Settings Reference

| Setting | Description |
|---------|-------------|
| VPN URL | The portal URL. Default: `https://vpn.ku.edu.tr` |
| Domain | Domain for DSID matching. Default: `vpn.ku.edu.tr` |
| Email | Pre-fill your university email for faster auto-login |
| Login mode | Full Auto / Visual Auto / Manual (slider) |
| OpenConnect path | Path to the `openconnect` binary. Leave blank to auto-detect. |
| Escalation tool | `sudo` or `pkexec` (Linux/macOS only) |
| Log level | Controls how much is shown in the Console tab |
| Close to tray | Minimise to tray instead of quitting when the window is closed |
| Client-side decorations | Use custom window chrome instead of the OS title bar |

Click **Reset to defaults** to restore all settings.

---

## Troubleshooting

### "Cookie was rejected by server"

Your saved session has expired. Click **Wipe Session** in the Connection tab, then reconnect.

### Auto-login fails / stuck in a login loop

Switch to **Manual** mode in Settings, connect, and complete the login yourself. Once the session is saved, switch back to Full Auto.

### OpenConnect shows a red X in Settings

OpenConnect is not installed or is not in the expected location. Install it:

- **Linux:** `sudo apt install openconnect` (or `dnf`, `pacman`, etc.)
- **macOS:** `brew install openconnect`
- **Windows:** Reinstall using the Setup installer from the Releases page.

Or enter the full path to your `openconnect` binary in the OpenConnect path field and click **Test**.

### App won't open on macOS ("app is damaged")

Run this command in Terminal:

```bash
sudo xattr -r -d com.apple.quarantine /Applications/KUVPN.app
```

This is a one-time step required because the app is not notarized through the Apple App Store.

### Logs

The **Console** tab shows real-time output from the login and VPN process. Increase the log level slider in Settings to `debug` or `trace` for more detail. Use **Copy Logs** to copy everything to your clipboard for bug reports.
