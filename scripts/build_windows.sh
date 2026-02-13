#!/bin/bash
set -e

# Configuration
APP_NAME="KUVPN"
VERSION="2.0.2"
OPENCONNECT_URL="https://github.com/openconnect/openconnect-gui/releases/download/v1.5.3/openconnect-gui-1.5.3-win32.exe"
INNO_SETUP_URL="https://files.jrsoftware.org/is/6/innosetup-6.2.2.exe"

# Podman detection
if [ "$1" != "--no-container" ] && [ ! -f /.containerenv ]; then
    if command -v podman >/dev/null 2>&1; then
        echo "Using Podman to build full Windows Installer..."
        podman build -t kuvpn-windows-builder -f packaging/windows/Dockerfile .
        
        CONTAINER_ID=$(podman create kuvpn-windows-builder)
        mkdir -p dist/windows
        podman cp "$CONTAINER_ID":/build/packaging/windows/KUVPN-Setup.exe dist/windows/
        podman cp "$CONTAINER_ID":/build/target/x86_64-pc-windows-gnu/release/kuvpn-gui.exe dist/windows/
        podman rm "$CONTAINER_ID"
        
        echo "Successfully built Windows installer in dist/windows/KUVPN-Setup.exe"
        exit 0
    fi
fi

if [ "$1" == "--no-container" ]; then
    shift
fi

echo "Building $APP_NAME for Windows..."

# 1. Cross-compile Rust binaries
rustup target add x86_64-pc-windows-gnu
cargo build -p kuvpn-gui --release --target x86_64-pc-windows-gnu

# 2. Download and prepare OpenConnect
echo "Preparing OpenConnect binaries..."
OC_DIR="packaging/windows/openconnect"
mkdir -p "$OC_DIR"

if [ ! -f "$OC_DIR/openconnect.exe" ]; then
    echo "Downloading OpenConnect bundle..."
    # We use the openconnect-gui installer and extract it
    wget -q -O /tmp/oc_setup.exe "$OPENCONNECT_URL"
    # Use 7z to extract if available, else skip bundling for now
    if command -v 7z >/dev/null 2>&1; then
        7z e -y /tmp/oc_setup.exe -o"$OC_DIR" "openconnect.exe" "*.dll" || true
    else
        echo "Warning: 7z not found. Skipping OpenConnect bundling."
    fi
fi

# 3. Generate Installer with Inno Setup (requires Wine)
if command -v wine >/dev/null 2>&1; then
    echo "Generating Windows Installer with Inno Setup..."
    
    # Download and install Inno Setup
    ISCC_PATH="/root/.wine/drive_c/Program Files (x86)/Inno Setup 6/ISCC.exe"
    if [ ! -f "$ISCC_PATH" ]; then
        echo "Inno Setup not found. Installing via Wine (with Xvfb)..."
        wget -q -O /tmp/is_setup.exe "$INNO_SETUP_URL"
        # Run silent install with Xvfb
        xvfb-run -a env WINEDEBUG=-all wine /tmp/is_setup.exe /VERYSILENT /SUPPRESSMSGBOXES /ALLUSERS /NOICONS || true
        # Wait for ISCC
        for i in {1..15}; do
            [ -f "$ISCC_PATH" ] && break
            sleep 2
        done
    fi

    # If ISCC is available on host or in container
    if [ -f "$ISCC_PATH" ] || command -v iscc >/dev/null 2>&1; then
        ISCC_CMD="iscc"
        [ -f "$ISCC_PATH" ] && ISCC_CMD="WINEDEBUG=-all wine \"$ISCC_PATH\""
        
        eval $ISCC_CMD packaging/windows/kuvpn.iss
    else
        echo "Warning: ISCC not found. Skipping installer generation."
        echo "You can manually run Inno Setup on packaging/windows/kuvpn.iss"
    fi
else
    echo "Warning: Wine not found. Skipping installer generation."
fi

echo "Done!"