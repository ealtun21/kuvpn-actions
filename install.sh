#!/bin/bash
# KUVPN Installer

set -e

# --- Configuration ---
REPO="KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="kuvpn"
GUI_NAME="KUVPN"
VERSION="${VERSION:-latest}"
INSTALL_CLI=0
INSTALL_GUI=0

# --- Colors ---
if [ -t 1 ]; then
    COLOR_PRIMARY="\033[0;34m"
    COLOR_WARN="\033[1;33m"
    COLOR_SUCCESS="\033[0;32m"
    COLOR_FAILURE="\033[0;31m"
    COLOR_RESET="\033[0m"
else
    COLOR_PRIMARY=""
    COLOR_WARN=""
    COLOR_SUCCESS=""
    COLOR_FAILURE=""
    COLOR_RESET=""
fi

log_info()    { printf "${COLOR_PRIMARY}[INFO]${COLOR_RESET} %s\n" "$1"; }
log_warn()    { printf "${COLOR_WARN}[WARN]${COLOR_RESET} %s\n" "$1"; }
log_success() { printf "${COLOR_SUCCESS}[OK]${COLOR_RESET} %s\n" "$1"; }
log_fail()    { printf "${COLOR_FAILURE}[FAIL]${COLOR_RESET} %s\n" "$1"; exit 1; }

# --- Prompt helper ---
prompt_yn() {
    # Usage: prompt_yn "Question?" [default: y|n]
    local question="$1"
    local default="${2:-y}"
    local prompt_str
    if [ "$default" = "y" ]; then
        prompt_str="[Y/n]"
    else
        prompt_str="[y/N]"
    fi
    while true; do
        printf "${COLOR_PRIMARY}?${COLOR_RESET} %s %s: " "$question" "$prompt_str"
        read -r answer
        answer="${answer:-$default}"
        case "$answer" in
            [Yy]*) return 0 ;;
            [Nn]*) return 1 ;;
            *) echo "  Please answer yes or no." ;;
        esac
    done
}

# --- Architecture Detection ---
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Darwin)
            if [ "$ARCH" = "x86_64" ]; then
                GUI_PLATFORM="macOS-x86_64"
                CLI_ARCH="x86_64"
            elif [ "$ARCH" = "arm64" ]; then
                GUI_PLATFORM="macOS-aarch64"
                CLI_ARCH="aarch64"
            else
                log_fail "Unsupported macOS architecture: $ARCH"
            fi
            ;;
        Linux)
            if [ "$ARCH" = "x86_64" ]; then
                GUI_PLATFORM="x86_64"
                CLI_ARCH="x86_64"
            elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
                GUI_PLATFORM="aarch64"
                CLI_ARCH="aarch64"
            else
                log_fail "Unsupported Linux architecture: $ARCH"
            fi
            ;;
        *)
            log_fail "Unsupported OS: $OS"
            ;;
    esac
}

# --- Version Resolution ---
resolve_version() {
    if [ "$VERSION" = "latest" ]; then
        log_info "Resolving latest version..."
        LATEST_URL="https://github.com/$REPO/releases/latest"
        if command -v curl >/dev/null 2>&1; then
            TAG=$(curl -sL -o /dev/null -w '%{url_effective}' "$LATEST_URL" | rev | cut -d/ -f1 | rev)
        else
            log_fail "curl is required to resolve the latest version."
        fi
    else
        TAG="$VERSION"
    fi

    if [ -z "$TAG" ]; then
        log_fail "Unable to resolve version."
    fi

    log_info "Selected version: $TAG"
}

# --- OpenConnect Detection (mirrors source code paths) ---
OPENCONNECT_SEARCH_PATHS=(
    "/sbin/openconnect"
    "/usr/sbin/openconnect"
    "/usr/local/sbin/openconnect"
    "/usr/local/bin/openconnect"
    "/opt/homebrew/bin/openconnect"
)

detect_openconnect() {
    # First check $PATH
    if command -v openconnect >/dev/null 2>&1; then
        command -v openconnect
        return 0
    fi
    # Then check the same fallback paths the app uses
    for p in "${OPENCONNECT_SEARCH_PATHS[@]}"; do
        if [ -f "$p" ] && [ -x "$p" ]; then
            echo "$p"
            return 0
        fi
    done
    return 1
}

# --- Package Manager Detection ---
detect_package_manager() {
    if command -v apt-get >/dev/null 2>&1; then echo "apt"
    elif command -v dnf >/dev/null 2>&1; then echo "dnf"
    elif command -v yum >/dev/null 2>&1; then echo "yum"
    elif command -v pacman >/dev/null 2>&1; then echo "pacman"
    elif command -v zypper >/dev/null 2>&1; then echo "zypper"
    elif command -v brew >/dev/null 2>&1; then echo "brew"
    else echo "none"
    fi
}

# --- OpenConnect Installation ---
install_openconnect_linux() {
    local pkg_mgr
    pkg_mgr=$(detect_package_manager)
    case "$pkg_mgr" in
        apt)
            log_info "Installing openconnect via apt..."
            sudo apt-get update -qq && sudo apt-get install -y openconnect
            ;;
        dnf)
            log_info "Installing openconnect via dnf..."
            sudo dnf install -y openconnect
            ;;
        yum)
            log_info "Installing openconnect via yum..."
            sudo yum install -y openconnect
            ;;
        pacman)
            log_info "Installing openconnect via pacman..."
            sudo pacman -Sy --noconfirm openconnect
            ;;
        zypper)
            log_info "Installing openconnect via zypper..."
            sudo zypper install -y openconnect
            ;;
        none)
            log_warn "Could not detect a supported package manager."
            echo "  Please install openconnect manually. KUVPN looks for it in:"
            echo "  \$PATH, /sbin, /usr/sbin, /usr/local/sbin, /usr/local/bin"
            echo "  Once installed, you can configure the path in the app's settings."
            ;;
        *)
            log_warn "Package manager '$pkg_mgr' not supported for automatic openconnect install."
            echo "  Please install openconnect manually."
            ;;
    esac
}

install_openconnect_macos() {
    # Check for Homebrew
    if command -v brew >/dev/null 2>&1; then
        if prompt_yn "Install openconnect via Homebrew?"; then
            log_info "Installing openconnect via Homebrew..."
            brew install openconnect
            log_success "openconnect installed via Homebrew."
        fi
    else
        log_warn "Homebrew is not installed."
        if prompt_yn "Install Homebrew first (then openconnect)?"; then
            log_info "Installing Homebrew..."
            /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
            log_success "Homebrew installed."
            log_info "Installing openconnect via Homebrew..."
            brew install openconnect
            log_success "openconnect installed."
        else
            log_warn "Skipping openconnect installation."
            echo "  KUVPN looks for openconnect in: \$PATH, /usr/local/bin, /opt/homebrew/bin"
            echo "  If you install it later, configure the path in the app's settings."
        fi
    fi
}

check_and_prompt_openconnect() {
    local oc_path
    if oc_path=$(detect_openconnect 2>/dev/null); then
        log_success "openconnect found at: $oc_path"
        return 0
    fi

    log_warn "openconnect was not found on this system."
    echo "  openconnect is required to establish VPN connections."
    echo ""

    if [ "$OS" = "Darwin" ]; then
        if prompt_yn "Would you like to install openconnect now?"; then
            install_openconnect_macos
        else
            echo "  You can install it later with: brew install openconnect"
        fi
    elif [ "$OS" = "Linux" ]; then
        local pkg_mgr
        pkg_mgr=$(detect_package_manager)
        if [ "$pkg_mgr" != "none" ]; then
            if prompt_yn "Would you like to install openconnect via $pkg_mgr?"; then
                install_openconnect_linux
            else
                echo "  Install openconnect using your package manager, e.g.:"
                case "$pkg_mgr" in
                    apt)     echo "    sudo apt-get install openconnect" ;;
                    dnf|yum) echo "    sudo $pkg_mgr install openconnect" ;;
                    pacman)  echo "    sudo pacman -S openconnect" ;;
                    zypper)  echo "    sudo zypper install openconnect" ;;
                esac
                echo "  KUVPN will look for it in \$PATH and common sbin directories."
            fi
        else
            install_openconnect_linux  # will print the manual install message
        fi
    fi
}

# --- Download Logic ---
install_cli_binary() {
    local download_url
    if [ "$OS" = "Darwin" ]; then
        download_url="https://github.com/$REPO/releases/download/${TAG}/${BINARY_NAME}-macos-${CLI_ARCH}"
    else
        download_url="https://github.com/$REPO/releases/download/${TAG}/${BINARY_NAME}-linux-${CLI_ARCH}"
    fi

    local tmp_dir
    tmp_dir=$(mktemp -d)
    local tmp_file="$tmp_dir/$BINARY_NAME"

    log_info "Downloading CLI from: $download_url"

    if ! curl --proto '=https' --tlsv1.2 -sSfL "$download_url" -o "$tmp_file"; then
        rm -rf "$tmp_dir"
        log_fail "Download failed. Check your internet connection and try again."
    fi

    mkdir -p "$INSTALL_DIR"
    mv "$tmp_file" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    rm -rf "$tmp_dir"

    log_success "CLI installed at $INSTALL_DIR/$BINARY_NAME"
}

install_gui_binary() {
    if [ "$OS" = "Darwin" ]; then
        local dmg_url="https://github.com/$REPO/releases/download/${TAG}/${GUI_NAME}-${GUI_PLATFORM}.dmg"
        local dest="$HOME/Downloads/${GUI_NAME}-${TAG}.dmg"
        log_info "Downloading macOS GUI (DMG) from: $dmg_url"
        curl --proto '=https' --tlsv1.2 -sSfL "$dmg_url" -o "$dest"
        log_success "GUI DMG downloaded to $dest"
        echo ""
        echo "  Next steps:"
        echo "  1. Open the DMG: open \"$dest\""
        echo "  2. Drag KUVPN.app to your Applications folder."
        echo "  3. Run this command to allow the app (macOS security requirement):"
        echo ""
        echo "     sudo xattr -r -d com.apple.quarantine /Applications/KUVPN.app"
        echo ""
        echo "  4. Open KUVPN from your Applications folder or Launchpad."
        echo ""
        if prompt_yn "Open the DMG now?"; then
            open "$dest"
        fi
    else
        local appimage_url="https://github.com/$REPO/releases/download/${TAG}/${GUI_NAME}-${GUI_PLATFORM}.AppImage"
        local dest="$INSTALL_DIR/${GUI_NAME}.AppImage"
        log_info "Downloading Linux GUI (AppImage) from: $appimage_url"
        mkdir -p "$INSTALL_DIR"
        if ! curl --proto '=https' --tlsv1.2 -sSfL "$appimage_url" -o "$dest"; then
            log_fail "Download failed. Check your internet connection and try again."
        fi
        chmod +x "$dest"
        log_success "GUI installed at $dest"

        # Create desktop entry
        local desktop_file="$HOME/.local/share/applications/kuvpn.desktop"
        mkdir -p "$(dirname "$desktop_file")"
        cat > "$desktop_file" <<EOF
[Desktop Entry]
Name=KUVPN
Exec=$dest
Icon=network-vpn
Type=Application
Categories=Network;
Comment=Connect to Koç University VPN
EOF
        log_success "Created desktop entry at $desktop_file"
    fi
}

# --- Shell Configuration ---
update_shell_config() {
    # Check if ~/.local/bin is already in PATH
    if echo "$PATH" | grep -q "$HOME/.local/bin"; then
        return 0
    fi

    log_warn "~/.local/bin is not in your PATH. Adding to shell configuration..."

    local path_str="\$HOME/.local/bin"
    local sh_cmd="export PATH=\"$path_str:\$PATH\""
    local fish_cmd="set -gx PATH $path_str \$PATH"
    local marker="# Added by kuvpn installer"
    local updated=0

    local files=("$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.profile")
    for config_file in "${files[@]}"; do
        if [ -f "$config_file" ]; then
            if ! grep -qF "$marker" "$config_file" && ! grep -qF 'export PATH="$HOME/.local/bin:$PATH"' "$config_file"; then
                { echo ""; echo "$marker"; echo "$sh_cmd"; } >> "$config_file"
                log_success "Added to PATH in $config_file"
                updated=1
            fi
        fi
    done

    local fish_config="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"
    if [ -d "$(dirname "$fish_config")" ]; then
        touch "$fish_config"
        if ! grep -qF "$marker" "$fish_config" && ! grep -qF 'set -gx PATH $HOME/.local/bin $PATH' "$fish_config"; then
            { echo ""; echo "$marker"; echo "$fish_cmd"; } >> "$fish_config"
            log_success "Added to PATH in $fish_config"
            updated=1
        fi
    fi

    if [ "$updated" -eq 1 ]; then
        log_warn "Please restart your terminal or source your shell config for PATH changes to take effect."
    fi
}

# --- Interactive Selection ---
interactive_select() {
    echo ""
    echo "What would you like to install?"
    echo "  1) CLI only  (command-line, run with: kuvpn)"
    echo "  2) GUI only  (graphical app with system tray)"
    echo "  3) Both CLI and GUI"
    echo ""
    while true; do
        printf "${COLOR_PRIMARY}?${COLOR_RESET} Enter choice [1/2/3]: "
        read -r choice
        case "$choice" in
            1) INSTALL_CLI=1; INSTALL_GUI=0; break ;;
            2) INSTALL_CLI=0; INSTALL_GUI=1; break ;;
            3) INSTALL_CLI=1; INSTALL_GUI=1; break ;;
            *) echo "  Please enter 1, 2, or 3." ;;
        esac
    done
}

# --- Main Execution ---

echo ""
log_info "KUVPN Installer"
echo ""

# Parse arguments
if [ $# -eq 0 ]; then
    interactive_select
else
    for arg in "$@"; do
        case $arg in
            --cli)         INSTALL_CLI=1 ;;
            --gui)         INSTALL_GUI=1 ;;
            --all)         INSTALL_CLI=1; INSTALL_GUI=1 ;;
            --version=*)   VERSION="${arg#*=}" ;;
            -y|--yes|--force) ;; # legacy flag, no-op now
        esac
    done
    # Fallback: no install target specified, default to CLI
    if [ "$INSTALL_CLI" -eq 0 ] && [ "$INSTALL_GUI" -eq 0 ]; then
        INSTALL_CLI=1
    fi
fi

detect_platform
resolve_version

if [ "$INSTALL_CLI" -eq 1 ]; then
    install_cli_binary
fi

if [ "$INSTALL_GUI" -eq 1 ]; then
    install_gui_binary
fi

if [ "$INSTALL_CLI" -eq 1 ] || [ "$INSTALL_GUI" -eq 1 ]; then
    update_shell_config
fi

echo ""
check_and_prompt_openconnect

echo ""
log_success "Done!"
if [ "$INSTALL_CLI" -eq 1 ]; then
    echo "  Run 'kuvpn' to use the CLI."
fi
if [ "$INSTALL_GUI" -eq 1 ] && [ "$OS" = "Linux" ]; then
    echo "  You can find KUVPN in your application menu, or run the AppImage directly."
fi
echo ""
