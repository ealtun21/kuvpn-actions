# KUVPN v3.0.0

KUVPN is a VPN client for Koç University that automates the Microsoft Azure AD / MFA browser login to retrieve a DSID cookie, then hands off to OpenConnect to establish the VPN tunnel.

> **KUVPN** is the graphical app (system tray, GUI window).
> **kuvpn** is the command-line tool (run `kuvpn` in a terminal).
> Both connect to the same VPN — pick whichever fits your workflow.

---

## Documentation

<table>
<tr>
<td align="center" width="33%">

### [Install](https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn#installation)
Get up and running on any platform

</td>
<td align="center" width="33%">

### [GUI Docs](docs/gui.md)
Graphical app — KUVPN

</td>
<td align="center" width="33%">

### [CLI Docs](docs/cli.md)
Command-line tool — kuvpn

</td>
</tr>
</table>

---

## Installation

**Linux / macOS:**

```bash
curl -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash
```

The script will ask what to install (GUI, CLI, or both), set up your PATH, and check for OpenConnect.

**Windows:** Download and run [`KUVPN-Setup-windows-x86_64.exe`](https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/releases/latest) from the Releases page.

<details><summary>Non-interactive flags (for scripting / automation)</summary>

```bash
# Install both
curl -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash -s -- --all

# GUI only
curl -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash -s -- --gui

# CLI only
curl -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash -s -- --cli
```

</details>

---

## Prerequisites

- **OpenConnect** must be installed to establish VPN connections.
  - The installer will offer to install it for you.
  - On Windows it is bundled inside the installer — nothing extra needed.
  - On Linux/macOS you can also install it manually: `sudo apt install openconnect` / `brew install openconnect`.

---

## Features

- Modern GUI with system tray and status icons
- OS notifications for MFA codes and connection events
- Auto-shows the window when input or MFA is required
- Full Auto, Visual Auto, and Manual login modes
- Headless (background) browser for seamless authentication
- Session persistence — stays logged in across reconnects
- Static CLI binary, no runtime dependencies

---

## License

MIT — see [LICENSE](LICENSE).

## Contributing

Issues and pull requests are welcome.
