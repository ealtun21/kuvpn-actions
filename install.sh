#!/bin/bash
# Installation script for KUVPN
# This script will download KUVPN and install it in $HOME/.kuvpn/bin
# It will also add $HOME/.kuvpn/bin to PATH for common shells.

COLOR_PRIMARY="\033[0;34m"
COLOR_WARN="\033[1;33m"
COLOR_SUCCESS="\033[0;32m"
COLOR_FAILURE="\033[0;31m"
COLOR_RESET="\033[0m"

TAG="v0.6.4"

echo ""
printf "${COLOR_PRIMARY}Installing KUVPN${COLOR_RESET}\n\n"
printf "This script will download KUVPN and install it in \$HOME/.kuvpn/bin\n\n"

CLI_DOWNLOAD_URL=""

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        if [ "$ARCH" = "x86_64" ]; then
            CLI_DOWNLOAD_URL="https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/releases/download/${TAG}/kuvpn-x86_64-apple-darwin"
        elif [ "$ARCH" = "arm64" ]; then
            CLI_DOWNLOAD_URL="https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/releases/download/${TAG}/kuvpn-aarch64-apple-darwin"
        else
            printf "${COLOR_FAILURE}Unsupported architecture: $ARCH${COLOR_RESET}\n"
            exit 1
        fi
        ;;
    Linux)
        if [ "$ARCH" = "x86_64" ]; then
            CLI_DOWNLOAD_URL="https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn/releases/download/${TAG}/kuvpn-x86_64-unknown-linux-musl"
        else
            printf "${COLOR_FAILURE}Unsupported architecture: $ARCH${COLOR_RESET}\n"
            exit 1
        fi
        ;;
    *)
        printf "${COLOR_FAILURE}Unsupported OS: $OS${COLOR_RESET}\n"
        exit 1
        ;;
esac

# Create the directory if it doesn't exist
if [ ! -d "$HOME/.kuvpn/bin" ]; then
    mkdir -p "$HOME/.kuvpn/bin" || {
        printf "${COLOR_FAILURE}Failed to create directory: $HOME/.kuvpn/bin${COLOR_RESET}\n"
        exit 1
    }
fi

# If there's already a kuvpn binary, ask before overwriting
if [ -f "$HOME/.kuvpn/bin/kuvpn" ]; then
    printf "${COLOR_WARN}A KUVPN binary already exists at $HOME/.kuvpn/bin/kuvpn${COLOR_RESET}\n"
    printf "Do you want to overwrite it? [y/N] "
    read -r OVERWRITE
    if [ "$OVERWRITE" != "y" ] && [ "$OVERWRITE" != "Y" ]; then
        printf "${COLOR_WARN}Installation aborted.${COLOR_RESET}\n"
        exit 1
    fi
fi

# Download function with fallback
download_kuvpn() {
    # Attempt curl first
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSfL "$CLI_DOWNLOAD_URL" -o "$HOME/.kuvpn/bin/kuvpn"
    # Fallback to wget if curl doesn't exist
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$HOME/.kuvpn/bin/kuvpn" "$CLI_DOWNLOAD_URL"
    else
        printf "${COLOR_FAILURE}Neither curl nor wget is installed. Please install one and retry.${COLOR_RESET}\n"
        exit 1
    fi
}

# Download the CLI
printf "${COLOR_PRIMARY}Downloading KUVPN from:${COLOR_RESET} $CLI_DOWNLOAD_URL\n\n"
download_kuvpn || {
    printf "${COLOR_FAILURE}Download failed!${COLOR_RESET}\n\n"
    exit 1
}

chmod +x "$HOME/.kuvpn/bin/kuvpn"

################################################################################
# Shell config updates
################################################################################

# Detect the user's shell (basename of $SHELL)
USER_SHELL="$(basename "$SHELL")"

# We’ll define the lines that we might need to add depending on shell
BASH_EXPORT_LINE='export PATH="$PATH:$HOME/.kuvpn/bin"'
ZSH_EXPORT_LINE='export PATH="$PATH:$HOME/.kuvpn/bin"'
FISH_EXPORT_LINE='set -gx PATH $PATH $HOME/.kuvpn/bin'

# A helper function to safely append lines
safe_append_line() {
    local config_file="$1"
    local line_to_add="$2"
    # Use grep with -Fxq to do an exact line match
    if [ -f "$config_file" ] && [ -w "$config_file" ] && ! grep -Fxq "$line_to_add" "$config_file"; then
        echo "$line_to_add" >> "$config_file"
        return 0
    fi
    return 1
}

# Keep track of whether we successfully updated any shell config
UPDATED_SHELL_CONFIG=false

case "$USER_SHELL" in
    bash)
        # Try appending to .bashrc, .bash_profile
        if safe_append_line "$HOME/.bashrc" "$BASH_EXPORT_LINE"; then
            UPDATED_SHELL_CONFIG=true
        fi
        if safe_append_line "$HOME/.bash_profile" "$BASH_EXPORT_LINE"; then
            UPDATED_SHELL_CONFIG=true
        fi
        ;;
    zsh)
        if safe_append_line "$HOME/.zshrc" "$ZSH_EXPORT_LINE"; then
            UPDATED_SHELL_CONFIG=true
        fi
        ;;
    fish)
        # fish typically uses ~/.config/fish/config.fish
        FISH_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"
        if [ ! -d "$(dirname "$FISH_CONFIG")" ]; then
            mkdir -p "$(dirname "$FISH_CONFIG")"
        fi
        if safe_append_line "$FISH_CONFIG" "$FISH_EXPORT_LINE"; then
            UPDATED_SHELL_CONFIG=true
        fi
        ;;
    *)
        # If the shell is something else, try updating .bashrc, .zshrc, etc. anyway
        if safe_append_line "$HOME/.bashrc" "$BASH_EXPORT_LINE"; then
            UPDATED_SHELL_CONFIG=true
        fi
        if safe_append_line "$HOME/.bash_profile" "$BASH_EXPORT_LINE"; then
            UPDATED_SHELL_CONFIG=true
        fi
        if safe_append_line "$HOME/.zshrc" "$ZSH_EXPORT_LINE"; then
            UPDATED_SHELL_CONFIG=true
        fi

        # If fish is installed, try that too
        FISH_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/fish/config.fish"
        if [ -d "$(dirname "$FISH_CONFIG")" ]; then
            if safe_append_line "$FISH_CONFIG" "$FISH_EXPORT_LINE"; then
                UPDATED_SHELL_CONFIG=true
            fi
        fi

        if [ "$UPDATED_SHELL_CONFIG" = false ]; then
            printf "${COLOR_WARN}Unsupported or unknown shell: $USER_SHELL. You may need to manually add $HOME/.kuvpn/bin to your PATH.${COLOR_RESET}\n"
        fi
        ;;
esac

if [ "$UPDATED_SHELL_CONFIG" = false ]; then
    printf "${COLOR_WARN}We couldn't automatically update your shell config, or it was already configured.${COLOR_RESET}\n"
    printf "${COLOR_WARN}You may need to manually ensure $HOME/.kuvpn/bin is in your PATH.${COLOR_RESET}\n"
fi

################################################################################
# Done
################################################################################

printf "\n${COLOR_SUCCESS}Installation complete!${COLOR_RESET}\n"
printf "KUVPN was installed to: $HOME/.kuvpn/bin/kuvpn\n\n"

printf "${COLOR_PRIMARY}Next Steps:${COLOR_RESET}\n"
printf "  1) Close and reopen your terminal, OR source your shell config (e.g. 'source ~/.bashrc')\n"
printf "  2) Run 'kuvpn --help' to get started.\n\n"
