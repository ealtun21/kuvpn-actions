#!/bin/bash
# KUVPN Installer

set -e

# --- Configuration ---
REPO="KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="kuvpn"
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
                PLATFORM="x86_64-unknown-linux-musl"
            elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
                PLATFORM="aarch64-unknown-linux-musl"
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
# Returns 0 if we should proceed, 1 if we should abort
check_existing_installation() {
    local BIN_PATH="$INSTALL_DIR/$BINARY_NAME"
    
    # If binary doesn't exist, proceed automatically
    if [ ! -f "$BIN_PATH" ]; then
        return 0
    fi

    # Check if force flag is on
    if [ "$FORCE_INSTALL" == "1" ]; then
        log_info "Force mode enabled. Overwriting existing installation."
        return 0
    fi

    # Try to extract current version
    # Output format expected: "kuvpn 2.0.0" -> awk prints "2.0.0"
    local CURRENT_VER_RAW=$("$BIN_PATH" --version 2>/dev/null || echo "unknown unknown")
    local CURRENT_VER=$(echo "$CURRENT_VER_RAW" | awk '{print $2}')
    
    # Clean 'v' from tag for comparison (v2.0.1 -> 2.0.1)
    local TARGET_VER=${TAG#v} 

    if [ "$CURRENT_VER" == "$TARGET_VER" ]; then
        # SAME VERSION
        log_warn "Version $TAG is already installed."
        if [ -t 0 ]; then
            printf "Re-install anyway? [y/N] "
            read -r REPLY
            if [[ ! "$REPLY" =~ ^[Yy]$ ]]; then
                log_warn "Aborted by user."
                return 1
            fi
        fi
    else
        # DIFFERENT VERSION
        log_warn "Existing installation found."
        printf "  Current: ${COLOR_PRIMARY}%s${COLOR_RESET}\n" "$CURRENT_VER"
        printf "  Target:  ${COLOR_PRIMARY}%s${COLOR_RESET}\n" "$TARGET_VER"
        
        if [ -t 0 ]; then
            printf "Do you want to update/replace? [Y/n] "
            read -r REPLY
            # Default to Yes if empty
            if [[ -n "$REPLY" && ! "$REPLY" =~ ^[Yy]$ ]]; then
                log_warn "Aborted by user."
                return 1
            fi
        fi
    fi

    return 0
}

# --- Download Logic ---
install_binary() {
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/${TAG}/${BINARY_NAME}-${PLATFORM}"
    TMP_DIR=$(mktemp -d)
    TMP_FILE="$TMP_DIR/$BINARY_NAME"

    log_info "Downloading from: $DOWNLOAD_URL"
    
    if command -v curl >/dev/null 2>&1; then
        if ! curl --proto '=https' --tlsv1.2 -sSfL "$DOWNLOAD_URL" -o "$TMP_FILE"; then
            rm -rf "$TMP_DIR"
            log_fail "Download failed."
        fi
    elif command -v wget >/dev/null 2>&1; then
        if ! wget -qO "$TMP_FILE" "$DOWNLOAD_URL"; then
            rm -rf "$TMP_DIR"
            log_fail "Download failed."
        fi
    else
        rm -rf "$TMP_DIR"
        log_fail "Neither curl nor wget found."
    fi

    mkdir -p "$INSTALL_DIR"
    mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    rm -rf "$TMP_DIR"
    
    log_success "Installed at $INSTALL_DIR/$BINARY_NAME"
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

    # 1. Update Standard Shells
    local FILES=("$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.zshrc" "$HOME/.profile")

    for config_file in "${FILES[@]}"; do
        if [ -f "$config_file" ]; then
            # Check for our marker comment OR the exact export line
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

    # 2. Update Fish Shell
    local FISH_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"
    if [ -d "$(dirname "$FISH_CONFIG")" ]; then
        touch "$FISH_CONFIG"
        # Check for our marker comment OR the exact fish PATH line
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
for arg in "$@"; do
    case $arg in
        -y|--yes|--force) FORCE_INSTALL=1 ;;
        --version=*) VERSION="${arg#*=}" ;;
    esac
done

detect_platform
resolve_version

# Check if we should proceed (version check happens here)
if check_existing_installation; then
    install_binary
    update_shell_config
    echo ""
    log_success "Installation Complete! Run 'kuvpn --help' to start."
    echo ""
else
    # We exit gracefully if check_existing_installation returns 1
    exit 0
fi
