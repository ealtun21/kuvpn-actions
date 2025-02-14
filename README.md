# KUVPN v0.6.6

KUVPN is a simple CLI tool written in Rust that retrieves the DSID cookie and executes the OpenConnect command to connect to the VPN for Koç University.


## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
  - [Binary Install (Recommended)](#binary-install-recommended)
  - [Binary Install (Manual)](#binary-install-manual)
  - [Build & Install from Source](#build--install-from-source)
- [Usage](#usage)
  - [Command-line Options](#command-line-options)
- [License](#license)
- [Contributing](#contributing)


## Features

- ✅ **Login** to vpn.ku.edu.tr on Linux/macOS.
- ✅ **Retrieve DSID cookie**
- ✅ **Execute OpenConnect command**
- ✅ **Customizable URL**
- ✅ **Secure session management**
- ✅ **Reliable Nix builds**
- ✅ **Debug logging**


## Prerequisites

- **Mandatory:** OpenConnect  
  - On Ubuntu:  
    ```bash
    sudo apt install openconnect
    ```
  - *Note:* Not required when using the `--dsid` / `-d` flag.

- **Optional:** Chromium/Chrome  
  *(Auto-downloaded if not found when needed.)*


## Installation

### Binary Install (Recommended)

Install KUVPN with the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash
```

> **Note:** Always inspect scripts before running commands from the internet!

### Binary Install (Manual)

1. **Download:** Get the latest binary from the [GitHub releases page](https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/releases).

2. **Move the binary:** Place it in a directory that's in your `$PATH` (e.g., `/usr/local/bin/`):

   ```bash
   sudo mv kuvpn /usr/local/bin/
   ```

3. **Make it executable:**

   ```bash
   sudo chmod +x /usr/local/bin/kuvpn
   ```

4. **Verify installation:**

   ```bash
   kuvpn --version
   ```

### Build & Install from Source

1. **Install Rustup:**  
   Use the following command (or your package manager) to install Rustup. Although Cargo is available through some package managers, using Rustup is recommended:

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install KUVPN from source:**

   ```bash
   cargo install kuvpn
   ```


## Usage

Simply run KUVPN with:

```bash
kuvpn
```

For help and more details:

```bash
kuvpn --help
```

To get additional runtime information (for example, logging), use:

```bash
kuvpn --level info
```

### Command-line Options

Below is the help message displayed when running `kuvpn --help`:

```text
Simple program to retrieve DSID cookie and execute OpenConnect command

Usage: kuvpn [OPTIONS]

Options:
  -u, --url <URL>
          The URL to the page where we will start logging in and looking for DSID
          [default: https://vpn.ku.edu.tr]

  -l, --level <LEVEL>
          The level of logging
          [default: error]
          Possible values:
            - off:   No logs
            - info:  Informational messages
            - warn:  Warning messages
            - debug: Debugging messages
            - error: Error messages
            - trace: Detailed stacktrace messages

  -d, --dsid
          Display the DSID without running OpenConnect

  -c, --clean
          Delete session information

  -a, --agent <AGENT>
          User agent for browser
          [default: Mozilla/5.0]

  -r, --run-command <RUN_COMMAND>
          Command to run OpenConnect with (e.g., doas, sudo, pkexec, or a custom script)

      --openconnect-path <OPENCONNECT_PATH>
          Path or command name for OpenConnect. Defaults to 'openconnect'. Can be a relative or absolute path
          [default: openconnect]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```



## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.



## Contributing

Contributions are welcome! If you have ideas, bug fixes, or improvements, please open an issue or submit a pull request.
