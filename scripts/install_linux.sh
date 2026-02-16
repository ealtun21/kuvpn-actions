#!/bin/bash
# Install KUVPN GUI from a local build with desktop integration.
# Run from the repository root after: cargo build --release -p kuvpn-gui
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"

GUI_BIN="$REPO_ROOT/target/release/kuvpn-gui"
DESKTOP_FILE="$REPO_ROOT/packaging/linux/kuvpn.desktop"
ICON_FILE="$REPO_ROOT/packaging/linux/kuvpn.png"

if [ ! -f "$GUI_BIN" ]; then
    echo "Error: $GUI_BIN not found. Build first with:"
    echo "  cargo build --release -p kuvpn-gui"
    exit 1
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

echo "Installing kuvpn-gui to $BIN_DIR..."
cp "$GUI_BIN" "$BIN_DIR/kuvpn-gui"
chmod +x "$BIN_DIR/kuvpn-gui"

echo "Installing desktop file to $APP_DIR..."
cp "$DESKTOP_FILE" "$APP_DIR/kuvpn.desktop"

echo "Installing icon to $ICON_DIR..."
cp "$ICON_FILE" "$ICON_DIR/kuvpn.png"

# Update icon cache if available
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

# Update desktop database if available
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

# Check PATH
if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
    echo ""
    echo "Warning: ~/.local/bin is not in your PATH."
    echo "Add it to your shell config, e.g.:"
    echo '  export PATH="$HOME/.local/bin:$PATH"'
fi

echo ""
echo "Done! KUVPN GUI installed successfully."
echo "Launch from your application menu or run: kuvpn-gui"
