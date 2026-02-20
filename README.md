# KUVPN v2.0.2

KUVPN is a simple VPN client for Koç University that retrieves the DSID cookie and executes the OpenConnect command. It provides both a modern GUI and a powerful CLI.

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
  - [GUI Installation (Recommended)](#gui-installation-recommended)
  - [CLI Installation](#cli-installation)
  - [Build from Source](#build--install-from-source)
- [Usage](#usage)
  - [GUI Usage](#gui-usage)
  - [Command-line Options](#command-line-options)
- [License](#license)
- [Contributing](#contributing)


## Features

- ✅ **Modern GUI** for easy connection management.
- ✅ **System Tray** integration with status indicators.
- ✅ **OS Notifications** for important events (MFA, connection status).
- ✅ **Login** to vpn.ku.edu.tr on Windows, Linux, and macOS.
- ✅ **Retrieves DSID cookie** automatically.
- ✅ **Executes OpenConnect** with correct parameters.
- ✅ **Headless/Automatic** or Manual login modes.
- ✅ **Session Management** to remember your login.

## Prerequisites

- **Mandatory:** OpenConnect  
  - On Ubuntu: `sudo apt install openconnect`
  - On macOS: `brew install openconnect`
  - On Windows: Included in the installer.

## Installation

### GUI Installation (Recommended)

1. **Windows:** Download `KUVPN-Setup-x86_64.exe` from the [Releases](https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/releases) page and run it.
2. **Linux:** Download the `.AppImage` for your architecture (x86_64 or aarch64), make it executable (`chmod +x KUVPN-x86_64.AppImage`), and run it.
3. **macOS:** Download the `.dmg` for your architecture, open it, and drag KUVPN to your Applications folder.

### CLI Installation

Install the KUVPN CLI with the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash
```

## Usage

### GUI Usage

Launch **KUVPN** from your application menu.
1. Enter your university email (optional, helps with auto-login).
2. Click **Connect**.
3. If prompted, complete the MFA authentication in the window or via your phone.
4. The app will stay in your system tray while connected.

### Command-line Options 

Simply run `kuvpn` in your terminal. For more options:

```bash
kuvpn --help
```


### Troubleshooting: Cookie Rejection

If you get this error, the easiest fix is to clear your session:
```
Unexpected 302 result from server
Creating SSL connection failed
Cookie was rejected by server; exiting.
```

You can clear your session like this:
```
kuvpn -c
```

### Troubleshooting: Unable to login?

If you aren't able to login via the cli'a automation, you can disable the auto login so that you can do it via the browser's gui.
To do this run the following command:
```
kuvpn --no-auto-login --disable-headless
```

Later, once you login, it will remember your session so you can login without the need for this, on the next logins.

### Waiting forever: seeminly doing nothing?
In this case, kuvpn may be trying to download a static chrome to run, you can check logs using `-l debug`, if after a long time it still fails to download & run chrome, you can install chrome/chromium by hand to the machine, to bypass this issue.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.



## Contributing

Contributions are welcome! If you have ideas, bug fixes, or improvements, please open an issue or submit a pull request.
