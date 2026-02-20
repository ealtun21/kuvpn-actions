#!/bin/bash
# KUVPN Installer

set -e

# --- Configuration ---
REPO="KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="kuvpn"
GUI_NAME="KUVPN"
VERSION="${VERSION:-latest}"
INSTALL_CLI=1
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

log_info() { printf "${COLOR_PRIMARY}[INFO]${COLOR_RESET} %s\n" "$1"; }
log_warn() { printf "${COLOR_WARN}[WARN]${COLOR_RESET} %s\n" "$1"; }
log_success() { printf "${COLOR_SUCCESS}[OK]${COLOR_RESET} %s\n" "$1"; }
log_fail() { printf "${COLOR_FAILURE}[FAIL]${COLOR_RESET} %s\n" "$1"; exit 1; }

# --- Architecture Detection ---
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Darwin)
            if [ "$ARCH" = "x86_64" ]; then
                GUI_PLATFORM="macOS-x86_64"
            elif [ "$ARCH" = "arm64" ]; then
                GUI_PLATFORM="macOS-aarch64"
            else
                log_fail "Unsupported macOS architecture: $ARCH"
            fi
            ;;
        Linux)
            if [ "$ARCH" = "x86_64" ]; then
                GUI_PLATFORM="x86_64"
            elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
                GUI_PLATFORM="aarch64"
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
            TAG=$(curl -sL -o /dev/null -w %{url_effective} "$LATEST_URL" | rev | cut -d/ -f1 | rev)
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

# --- Download Logic ---
install_cli_binary() {
    if [ "$OS" = "Darwin" ]; then
        DOWNLOAD_URL="https://github.com/$REPO/releases/download/${TAG}/${BINARY_NAME}-macos-${GUI_PLATFORM#macOS-}"
    else
        DOWNLOAD_URL="https://github.com/$REPO/releases/download/${TAG}/${BINARY_NAME}-linux-${GUI_PLATFORM}"
    fi

    TMP_DIR=$(mktemp -d)
    TMP_FILE="$TMP_DIR/$BINARY_NAME"

    log_info "Downloading CLI from: $DOWNLOAD_URL"
    
    if ! curl --proto '=https' --tlsv1.2 -sSfL "$DOWNLOAD_URL" -o "$TMP_FILE"; then
        rm -rf "$TMP_DIR"
        log_fail "Download failed."
    fi

    mkdir -p "$INSTALL_DIR"
    mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    rm -rf "$TMP_DIR"
    
    log_success "CLI installed at $INSTALL_DIR/$BINARY_NAME"
}

install_gui_binary() {
    if [ "$OS" = "Darwin" ]; then
        DOWNLOAD_URL="https://github.com/$REPO/releases/download/${TAG}/${GUI_NAME}-${GUI_PLATFORM}.dmg"
        log_info "Downloading macOS GUI (DMG) from: $DOWNLOAD_URL"
        DEST="$HOME/Downloads/${GUI_NAME}-${TAG}.dmg"
        curl -L "$DOWNLOAD_URL" -o "$DEST"
        log_success "GUI DMG downloaded to $DEST"
        log_warn "Please open the DMG and drag KUVPN to Applications."
    else
        DOWNLOAD_URL="https://github.com/$REPO/releases/download/${TAG}/${GUI_NAME}-${GUI_PLATFORM}.AppImage"
        log_info "Downloading Linux GUI (AppImage) from: $DOWNLOAD_URL"
        mkdir -p "$INSTALL_DIR"
        DEST="$INSTALL_DIR/${GUI_NAME}.AppImage"
        curl -L "$DOWNLOAD_URL" -o "$DEST"
        chmod +x "$DEST"
        log_success "GUI installed at $DEST"
        
        # Create Desktop Entry
        DESKTOP_FILE="$HOME/.local/share/applications/kuvpn.desktop"
        mkdir -p "$(dirname "$DESKTOP_FILE")"
        cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Name=KUVPN
Exec=$DEST
Icon=network-vpn
Type=Application
Categories=Network;
Comment=Connect to Koç University VPN
EOF
        log_success "Created desktop entry at $DESKTOP_FILE"
    fi
}

# --- Shell Configuration ---
update_shell_config() {
    # Check if ~/.local/bin is already in PATH
    if echo "$PATH" | grep -q "$HOME/.local/bin"; then
        return 0
    fi
    
    # Not in PATH, add it to shell configs
    log_warn "~/.local/bin is not in your PATH. Adding to shell configuration..."
    
    local PATH_STR="\$HOME/.local/bin"
    local SH_CMD="export PATH=\"$PATH_STR:\$PATH\""
    local FISH_CMD="set -gx PATH $PATH_STR \$PATH"
    local MARKER="# Added by kuvpn installer"
    local UPDATED=0

    # Update Standard Shells
    local FILES=("$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.profile")

    for config_file in "${FILES[@]}"; do
        if [ -f "$config_file" ]; then
            if ! grep -qF "$MARKER" "$config_file" && ! grep -qF 'export PATH="$HOME/.local/bin:$PATH"' "$config_file"; then
                echo "" >> "$config_file"
                echo "$MARKER" >> "$config_file"
                echo "$SH_CMD" >> "$config_file"
                log_success "Added to PATH in $config_file"
                UPDATED=1
            fi
        fi
    done

    # Update Fish Shell
    local FISH_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"
    if [ -d "$(dirname "$FISH_CONFIG")" ]; then
        touch "$FISH_CONFIG"
        if ! grep -qF "$MARKER" "$FISH_CONFIG" && ! grep -qF 'set -gx PATH $HOME/.local/bin $PATH' "$FISH_CONFIG"; then
            echo "" >> "$FISH_CONFIG"
            echo "$MARKER" >> "$FISH_CONFIG"
            echo "$FISH_CMD" >> "$FISH_CONFIG"
            log_success "Added to PATH in $FISH_CONFIG"
            UPDATED=1
        fi
    fi

    if [ $UPDATED -eq 1 ]; then
        log_info "Please restart your terminal or run: source <your_shell_config>"
    fi
}

# --- Main Execution ---

echo ""
log_info "KUVPN Installer"

# Default if no arguments provided: install CLI
if [ $# -eq 0 ]; then
    INSTALL_CLI=1
    INSTALL_GUI=0
else
    # If arguments are provided, we reset defaults and only install what is requested
    INSTALL_CLI=0
    INSTALL_GUI=0
    for arg in "$@"; do
        case $arg in
            --cli) INSTALL_CLI=1 ;;
            --gui) INSTALL_GUI=1 ;;
            --all) INSTALL_CLI=1; INSTALL_GUI=1 ;;
            --version=*) VERSION="${arg#*=}" ;;
            -y|--yes|--force) FORCE_INSTALL=1 ;; # Keep support for force flag
        esac
    done
    # Fallback: if user provided args but neither --cli, --gui, nor --all, default to CLI
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

update_shell_config

echo ""
log_success "Done!"
if [ "$INSTALL_CLI" -eq 1 ]; then
    echo "Run 'kuvpn' to use the CLI."
fi
if [ "$INSTALL_GUI" -eq 1 ] && [ "$OS" = "Linux" ]; then
    echo "You can find KUVPN in your application menu."
fi
echo ""
