#!/bin/bash
# KUVPN Installer
# Usage:
#   curl -sSfL <url>/install.sh | bash              # Install CLI only
#   curl -sSfL <url>/install.sh | bash -s -- --gui  # Install GUI with desktop integration

set -e

# --- Configuration ---
REPO="KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn"
INSTALL_DIR="$HOME/.local/bin"
VERSION="${VERSION:-latest}"

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
                PLATFORM="x86_64-apple-darwin"
            elif [ "$ARCH" = "arm64" ]; then
                PLATFORM="aarch64-apple-darwin"
            else
                log_fail "Unsupported macOS architecture: $ARCH"
            fi
            ;;
        Linux)
            if [ "$ARCH" = "x86_64" ]; then
                PLATFORM="linux-x86_64"
            elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
                PLATFORM="linux-aarch64"
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

# --- Pre-Install Check ---
check_existing_installation() {
    local BIN_NAME="$1"
    local BIN_PATH="$INSTALL_DIR/$BIN_NAME"

    if [ ! -f "$BIN_PATH" ]; then
        return 0
    fi

    if [ "$FORCE_INSTALL" == "1" ]; then
        log_info "Force mode enabled. Overwriting existing installation."
        return 0
    fi

    local CURRENT_VER_RAW=$("$BIN_PATH" --version 2>/dev/null || echo "unknown unknown")
    local CURRENT_VER=$(echo "$CURRENT_VER_RAW" | awk '{print $2}')
    local TARGET_VER=${TAG#v}

    if [ "$CURRENT_VER" == "$TARGET_VER" ]; then
        log_warn "$BIN_NAME version $TAG is already installed."
        if [ -t 0 ]; then
            printf "Re-install anyway? [y/N] "
            read -r REPLY
            if [[ ! "$REPLY" =~ ^[Yy]$ ]]; then
                log_warn "Aborted by user."
                return 1
            fi
        fi
    else
        log_warn "Existing $BIN_NAME installation found."
        printf "  Current: ${COLOR_PRIMARY}%s${COLOR_RESET}\n" "$CURRENT_VER"
        printf "  Target:  ${COLOR_PRIMARY}%s${COLOR_RESET}\n" "$TARGET_VER"

        if [ -t 0 ]; then
            printf "Do you want to update/replace? [Y/n] "
            read -r REPLY
            if [[ -n "$REPLY" && ! "$REPLY" =~ ^[Yy]$ ]]; then
                log_warn "Aborted by user."
                return 1
            fi
        fi
    fi

    return 0
}

# --- Download Helper ---
download_file() {
    local URL="$1"
    local DEST="$2"

    if command -v curl >/dev/null 2>&1; then
        if ! curl --proto '=https' --tlsv1.2 -sSfL "$URL" -o "$DEST"; then
            return 1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if ! wget -qO "$DEST" "$URL"; then
            return 1
        fi
    else
        log_fail "Neither curl nor wget found."
    fi
    return 0
}

# --- Install Binary ---
install_binary() {
    local BINARY_NAME="$1"
    local ASSET_NAME="$2"

    local DOWNLOAD_URL="https://github.com/$REPO/releases/download/${TAG}/${ASSET_NAME}"
    local TMP_DIR=$(mktemp -d)
    local TMP_FILE="$TMP_DIR/$BINARY_NAME"

    log_info "Downloading $BINARY_NAME from: $DOWNLOAD_URL"

    if ! download_file "$DOWNLOAD_URL" "$TMP_FILE"; then
        rm -rf "$TMP_DIR"
        log_fail "Download of $BINARY_NAME failed."
    fi

    mkdir -p "$INSTALL_DIR"
    mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    rm -rf "$TMP_DIR"

    log_success "Installed $BINARY_NAME at $INSTALL_DIR/$BINARY_NAME"
}

# --- Desktop Integration (Linux GUI only) ---
install_desktop_integration() {
    if [ "$OS" != "Linux" ]; then
        return 0
    fi

    local APP_DIR="$HOME/.local/share/applications"
    local ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
    local TMP_DIR=$(mktemp -d)

    mkdir -p "$APP_DIR" "$ICON_DIR"

    # Download desktop file and icon from the release
    local DESKTOP_URL="https://raw.githubusercontent.com/$REPO/${TAG}/packaging/linux/kuvpn.desktop"
    local ICON_URL="https://raw.githubusercontent.com/$REPO/${TAG}/packaging/linux/kuvpn.png"

    log_info "Installing desktop integration..."

    if download_file "$DESKTOP_URL" "$TMP_DIR/kuvpn.desktop"; then
        cp "$TMP_DIR/kuvpn.desktop" "$APP_DIR/kuvpn.desktop"
        log_success "Desktop file installed to $APP_DIR/"
    else
        # Fallback: create desktop file inline
        cat > "$APP_DIR/kuvpn.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=KUVPN
Comment=Connect to Koc University Network
Exec=kuvpn-gui
Icon=kuvpn
Categories=Network;
Terminal=false
EOF
        log_success "Desktop file created at $APP_DIR/"
    fi

    if download_file "$ICON_URL" "$TMP_DIR/kuvpn.png"; then
        cp "$TMP_DIR/kuvpn.png" "$ICON_DIR/kuvpn.png"
        log_success "Icon installed to $ICON_DIR/"
    else
        log_warn "Could not download icon. App may show without an icon in launchers."
    fi

    rm -rf "$TMP_DIR"

    # Update caches
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APP_DIR" 2>/dev/null || true
    fi
}

# --- Shell Configuration ---
update_shell_config() {
    if echo "$PATH" | grep -q "$HOME/.local/bin"; then
        return 0
    fi

    log_warn "~/.local/bin is not in your PATH. Adding to shell configuration..."

    local PATH_STR="\$HOME/.local/bin"
    local SH_CMD="export PATH=\"$PATH_STR:\$PATH\""
    local FISH_CMD="set -gx PATH $PATH_STR \$PATH"
    local MARKER="# Added by kuvpn installer"
    local UPDATED=0

    local FILES=("$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.profile")

    for config_file in "${FILES[@]}"; do
        if [ -f "$config_file" ]; then
            if ! grep -qF "$MARKER" "$config_file" && ! grep -qF 'export PATH="$HOME/.local/bin:$PATH"' "$config_file"; then
                echo "" >> "$config_file"
                echo "$MARKER" >> "$config_file"
                echo "$SH_CMD" >> "$config_file"
                log_success "Added to PATH in $config_file"
                UPDATED=1
            else
                log_info "Already in PATH: $config_file"
            fi
        fi
    done

    local FISH_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"
    if [ -d "$(dirname "$FISH_CONFIG")" ]; then
        touch "$FISH_CONFIG"
        if ! grep -qF "$MARKER" "$FISH_CONFIG" && ! grep -qF 'set -gx PATH $HOME/.local/bin $PATH' "$FISH_CONFIG"; then
            echo "" >> "$FISH_CONFIG"
            echo "$MARKER" >> "$FISH_CONFIG"
            echo "$FISH_CMD" >> "$FISH_CONFIG"
            log_success "Added to PATH in $FISH_CONFIG"
            UPDATED=1
        else
            log_info "Already in PATH: $FISH_CONFIG"
        fi
    fi

    echo ""
    if [ $UPDATED -eq 1 ]; then
        log_info "Please restart your terminal or run: source <your_shell_config>"
    fi
}

# --- Main Execution ---

echo ""
log_info "Starting KUVPN Installer..."

FORCE_INSTALL=0
INSTALL_GUI=0
for arg in "$@"; do
    case $arg in
        -y|--yes|--force) FORCE_INSTALL=1 ;;
        --gui) INSTALL_GUI=1 ;;
        --version=*) VERSION="${arg#*=}" ;;
    esac
done

detect_platform
resolve_version

if [ "$INSTALL_GUI" == "1" ]; then
    # GUI install
    if check_existing_installation "kuvpn-gui"; then
        install_binary "kuvpn-gui" "kuvpn-gui-${PLATFORM}"
        install_desktop_integration
        update_shell_config
        echo ""
        log_success "Installation Complete! Launch KUVPN from your application menu or run 'kuvpn-gui'."
        echo ""
    else
        exit 0
    fi
else
    # CLI install (default)
    if check_existing_installation "kuvpn-cli"; then
        install_binary "kuvpn-cli" "kuvpn-cli-${PLATFORM}"
        update_shell_config
        echo ""
        log_success "Installation Complete! Run 'kuvpn-cli --help' to start."
        echo ""
    else
        exit 0
    fi
fi
