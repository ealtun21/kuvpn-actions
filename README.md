# KUVPN v1.0.2

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
  - [Running on Headless](#running-on-headless)
- [License](#license)
- [Contributing](#contributing)


## Features

- ✅ **Login** to vpn.ku.edu.tr on Linux.
- ✅ **Retrieve DSID cookie**
- ✅ **Execute OpenConnect command**
- ✅ **Headless Execution**
- ✅ **Session management**
- ✅ **CLI Based**
- ✅ **Openconnect Wrapper**


## Prerequisites

- **Mandatory:** OpenConnect  
  - On Ubuntu:  
    ```bash
    sudo apt install openconnect
    ```

## Installation

### Binary Install (Recommended)

Install KUVPN with the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSfL https://raw.githubusercontent.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/main/install.sh | bash
```

> **Note:** Always inspect scripts before running commands from the internet!

### Binary Install (Manual) ( Please use KUVPN v0.7.0 for Mac OS ) 

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
   cargo install --git https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn
   ```


## Usage

Simply run KUVPN with: ( No arguments required )

```bash
kuvpn
```

For help and more details:

```bash
kuvpn --help
```

### Command-line Options

Below is the help message displayed when running `kuvpn --help`:

```text
A convenient tool for connecting to Koç University's VPN using OpenConnect.

Usage: kuvpn [OPTIONS]

Options:
  -u, --url <URL>
          The URL to the page where we will start logging in and looking for DSID [default: https://vpn.ku.edu.tr]
      --domain <DOMAIN>
          The Domain of the DSID found [default: vpn.ku.edu.tr]
  -g, --get-dsid
          Gives the user the dsid without running openconnect
  -d, --disable-headless
          Gets DSID without headless mode
  -c, --clean
          Delete session information
  -r, --run-command <RUN_COMMAND>
          Command to run openconnect with (e.g., doas, sudo, pkexec, or a custom script)
      --openconnect-path <OPENCONNECT_PATH>
          Path or command name for openconnect. Defaults to 'openconnect'. Can be a relative or absolute path [default: openconnect]
  -h, --help
          Print help
  -V, --version
          Print version
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

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.



## Contributing

Contributions are welcome! If you have ideas, bug fixes, or improvements, please open an issue or submit a pull request.
