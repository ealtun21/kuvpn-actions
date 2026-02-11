#!/bin/bash
set -e

# Configuration
APP_NAME="KUVPN"
VERSION="2.0.2"
ARCH="x86_64"

# Build the GUI binary
cargo build -p kuvpn-gui --release

# Setup AppDir
APPDIR="packaging/appimage/AppDir"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp target/release/kuvpn-gui "$APPDIR/usr/bin/"

# Create desktop file
cat > "$APPDIR/usr/share/applications/kuvpn.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=KUVPN GUI
Exec=kuvpn-gui
Icon=kuvpn
Categories=Network;
Comment=Connect to Koç University VPN
Terminal=false
EOF

# Create placeholder icon if needed
if [ ! -f "packaging/appimage/kuvpn.png" ]; then
    if command -v convert >/dev/null 2>&1; then
        convert -size 256x256 xc:blue packaging/appimage/kuvpn.png
    else
        touch packaging/appimage/kuvpn.png
    fi
fi

# Download linuxdeploy if needed
if [ ! -f "packaging/appimage/linuxdeploy" ]; then
    curl -L -o packaging/appimage/linuxdeploy https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
    chmod +x packaging/appimage/linuxdeploy
fi

# Download appimagetool if needed
if [ ! -f "packaging/appimage/appimagetool" ]; then
    curl -L -o packaging/appimage/appimagetool https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x packaging/appimage/appimagetool
fi

# Bundle openconnect if found on host
OPENCONNECT_HOST=$(which openconnect || true)
if [ -n "$OPENCONNECT_HOST" ]; then
    echo "Bundling openconnect from host: $OPENCONNECT_HOST"
    cp "$OPENCONNECT_HOST" "$APPDIR/usr/bin/"
fi

# Build Minimal AppImage
export NO_STRIP=1
./packaging/appimage/linuxdeploy --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/kuvpn-gui" \
    $( [ -f "$APPDIR/usr/bin/openconnect" ] && echo "--executable $APPDIR/usr/bin/openconnect" ) \
    --desktop-file "$APPDIR/usr/share/applications/kuvpn.desktop" \
    --icon-file packaging/appimage/kuvpn.png \
    --custom-apprun scripts/AppRun.sh

ARCH=$ARCH ./packaging/appimage/appimagetool --appimage-extract-and-run "$APPDIR" "${APP_NAME}-minimal-${ARCH}.AppImage"

# Build Full AppImage (Optional)
if [ "$1" == "--full" ]; then
    echo "Downloading Chromium for full AppImage..."
    REVISION=$(curl -s "https://www.googleapis.com/download/storage/v1/b/chromium-browser-snapshots/o/Linux_x64%2FLAST_CHANGE?alt=media")
    curl -L -o packaging/chromium.zip "https://www.googleapis.com/download/storage/v1/b/chromium-browser-snapshots/o/Linux_x64%2F${REVISION}%2Fchrome-linux.zip?alt=media"
    mkdir -p "$APPDIR/usr/lib/chromium"
    unzip -q packaging/chromium.zip -d packaging/appimage/AppDir/usr/lib/
    mv "$APPDIR/usr/lib/chrome-linux/"* "$APPDIR/usr/lib/chromium/"
    rm -rf "$APPDIR/usr/lib/chrome-linux" packaging/chromium.zip
    
    ARCH=$ARCH ./packaging/appimage/appimagetool --appimage-extract-and-run "$APPDIR" "${APP_NAME}-full-${ARCH}.AppImage"
fi

echo "Done!"
