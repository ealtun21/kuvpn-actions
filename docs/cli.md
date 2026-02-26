# kuvpn — CLI Documentation

[← Back to README](../README.md) · [GUI Documentation →](gui.md)

`kuvpn` is the command-line frontend for KUVPN. It automates the Microsoft Azure AD / MFA login flow in a headless browser, retrieves the DSID cookie, and launches OpenConnect to establish the VPN tunnel.

---

## Installation

**Recommended:**

```bash
curl -sSfL https://raw.githubusercontent.com/ealtun21/kuvpn-actions/main/install.sh | bash
```

The script will ask what to install — choose CLI. It downloads the right static binary for your platform, places it at `~/.local/bin/kuvpn`, adds it to your PATH if needed, and optionally installs OpenConnect.

<details><summary>Non-interactive (for scripting / automation)</summary>

```bash
curl -sSfL https://raw.githubusercontent.com/ealtun21/kuvpn-actions/main/install.sh | bash -s -- --cli
```

</details>

**Manual install:**

Download the binary for your platform from the [Releases](https://github.com/ealtun21/kuvpn-actions/releases/latest) page, make it executable, and move it onto your PATH:

```bash
# Linux x86_64
chmod +x kuvpn-linux-x86_64 && mv kuvpn-linux-x86_64 ~/.local/bin/kuvpn

# Linux aarch64
chmod +x kuvpn-linux-aarch64 && mv kuvpn-linux-aarch64 ~/.local/bin/kuvpn

# macOS Intel
chmod +x kuvpn-macos-x86_64 && mv kuvpn-macos-x86_64 ~/.local/bin/kuvpn

# macOS Apple Silicon
chmod +x kuvpn-macos-aarch64 && mv kuvpn-macos-aarch64 ~/.local/bin/kuvpn
```

---

## Basic Usage

```bash
kuvpn
```

That's it for most users. `kuvpn` will open a headless browser, complete the Azure AD / MFA login automatically, and start OpenConnect. You'll see a progress spinner and live log output while it connects.

Press **Ctrl+C** to disconnect.

---

## Options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--url` | `-u` | `https://vpn.ku.edu.tr` | VPN portal URL |
| `--domain` | | `vpn.ku.edu.tr` | Domain used for DSID cookie matching |
| `--email` | | *(none)* | Pre-fill your university email to speed up login |
| `--level` | `-l` | `error` | Log level: `off`, `error`, `warn`, `info`, `debug`, `trace` |
| `--disable-headless` | `-d` | `false` | Show the browser window instead of running headlessly |
| `--no-auto-login` | | `false` | Disable automation — poll for DSID in a visible browser you control |
| `--get-dsid` | `-g` | `false` | Print the DSID cookie and exit without starting OpenConnect |
| `--clean` | `-c` | `false` | Delete saved session data and exit |
| `--run-command` | `-r` | *(auto-detected)* | Override the privilege escalation tool (`sudo`, `pkexec`, or a custom script) |
| `--openconnect-path` | | `openconnect` | Path or command name for the OpenConnect binary |
| `--interface-name` | | `kuvpn0` | Name for the TUN interface created by OpenConnect |

---

## Login Modes

### Full Auto (default)
The browser runs headlessly. KUVPN detects each page of the login flow and fills in fields automatically. Works for most users who have their session cached.

### Visual Auto (`--disable-headless`)
Opens a visible browser window but still attempts to automate the login. Useful for debugging or when a CAPTCHA appears.

### Manual (`--no-auto-login --disable-headless`)
Opens a visible browser and does nothing — you complete the login yourself. KUVPN waits until it detects the DSID cookie, then takes over and starts OpenConnect.

```bash
kuvpn --no-auto-login --disable-headless
```

After you log in once this way, the session is saved and future runs can use Full Auto again.

---

## Troubleshooting

### Cookie rejected by server

```
Unexpected 302 result from server
Cookie was rejected by server; exiting.
```

Your saved session has expired. Clear it and reconnect:

```bash
kuvpn --clean
kuvpn
```

### Stuck at startup / downloading Chrome

`kuvpn` uses a headless Chromium browser. If it seems to hang on the first run it may be downloading Chromium. Check with:

```bash
kuvpn -l debug
```

If the download fails repeatedly, install Chrome or Chromium via your package manager — KUVPN will detect and use the system installation.

### Can't log in automatically

Try Manual mode so you can see what's happening:

```bash
kuvpn --no-auto-login --disable-headless
```

Once you complete the login manually, the session is saved for future runs.

### OpenConnect not found

Make sure OpenConnect is installed and on your PATH. The installer can do this for you, or install manually:

```bash
# Debian / Ubuntu
sudo apt install openconnect

# Fedora / RHEL
sudo dnf install openconnect

# Arch
sudo pacman -S openconnect

# macOS
brew install openconnect
```

If it's installed in a non-standard location, point `kuvpn` at it:

```bash
kuvpn --openconnect-path /usr/local/sbin/openconnect
```
