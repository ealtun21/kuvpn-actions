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
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp target/release/kuvpn-gui "$APPDIR/usr/bin/"

# Create desktop file
cat > "$APPDIR/usr/share/applications/kuvpn.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=KUVPN
Exec=kuvpn-gui
Icon=kuvpn
Categories=Network;
Comment=Connect to Koç University Network
Terminal=false
EOF

# Create icon from SVG if possible, else placeholder
if [ -f "crates/kuvpn-gui/assets/ku.svg" ] && command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w 256 -h 256 "crates/kuvpn-gui/assets/ku.svg" -o packaging/appimage/kuvpn.png
elif [ ! -f "packaging/appimage/kuvpn.png" ]; then
    if command -v convert >/dev/null 2>&1; then
        # Create a burgundy placeholder
        convert -size 256x256 xc:"#800020" packaging/appimage/kuvpn.png
    else
        touch packaging/appimage/kuvpn.png
    fi
fi

# Download linuxdeploy if needed
if [ ! -f "packaging/appimage/linuxdeploy" ]; then
    curl -L -o packaging/appimage/linuxdeploy https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
    chmod +x packaging/appimage/linuxdeploy
fi

# Download linuxdeploy-plugin-gtk if needed
if [ ! -f "packaging/appimage/linuxdeploy-plugin-gtk.sh" ]; then
    curl -L -o packaging/appimage/linuxdeploy-plugin-gtk.sh https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh
    chmod +x packaging/appimage/linuxdeploy-plugin-gtk.sh
fi

# Download appimagetool if needed
if [ ! -f "packaging/appimage/appimagetool" ]; then
    curl -L -o packaging/appimage/appimagetool https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x packaging/appimage/appimagetool
fi

# Build Minimal AppImage
export NO_STRIP=1
export DEPLOY_GTK_VERSION=3
# Ensure we include appindicator for the tray and pangoft2 (often missing in minimal setups)
export EXTRA_LIBS="libayatana-appindicator3.so.1;libappindicator3.so.1;libpangoft2-1.0.so.0"

# Make sure linuxdeploy can find the gtk plugin
cp packaging/appimage/linuxdeploy-plugin-gtk.sh ./linuxdeploy-plugin-gtk.sh

./packaging/appimage/linuxdeploy --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/kuvpn-gui" \
    --desktop-file "$APPDIR/usr/share/applications/kuvpn.desktop" \
    --icon-file packaging/appimage/kuvpn.png \
    --plugin gtk \
    --custom-apprun scripts/AppRun.sh

rm ./linuxdeploy-plugin-gtk.sh

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
