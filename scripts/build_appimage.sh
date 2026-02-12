#!/bin/bash
set -e

# Configuration
APP_NAME="KUVPN"
VERSION="2.0.2"
ARCH="x86_64"

# Podman/Docker detection and wrapper
if [ "$1" != "--no-container" ] && [ ! -f /.containerenv ] && [ ! -f /run/.containerenv ]; then
    BUILD_ARGS=""
    if [ "$1" == "--full" ]; then
        BUILD_ARGS="--full"
    fi

    if command -v podman >/dev/null 2>&1; then
        echo "Using Podman to build AppImage for maximum compatibility..."
        podman build --build-arg BUILD_ARGS="$BUILD_ARGS" -t kuvpn-builder -f packaging/appimage/Dockerfile .
        
        CONTAINER_ID=$(podman create kuvpn-builder)
        podman cp "$CONTAINER_ID":/build/${APP_NAME}-minimal-${ARCH}.AppImage .
        if [ "$1" == "--full" ]; then
             podman cp "$CONTAINER_ID":/build/${APP_NAME}-full-${ARCH}.AppImage .
        fi
        podman rm "$CONTAINER_ID"
        
        echo "Successfully built and extracted AppImage(s)."
        exit 0
    elif command -v docker >/dev/null 2>&1; then
        echo "Using Docker to build AppImage for maximum compatibility..."
        docker build --build-arg BUILD_ARGS="$BUILD_ARGS" -t kuvpn-builder -f packaging/appimage/Dockerfile .
        
        CONTAINER_ID=$(docker create kuvpn-builder)
        docker cp "$CONTAINER_ID":/build/${APP_NAME}-minimal-${ARCH}.AppImage .
        if [ "$1" == "--full" ]; then
             docker cp "$CONTAINER_ID":/build/${APP_NAME}-full-${ARCH}.AppImage .
        fi
        docker rm "$CONTAINER_ID"
        
        echo "Successfully built and extracted AppImage(s)."
        exit 0
    else
        echo "Neither Podman nor Docker found. Building on host (may have compatibility issues)..."
    fi
fi

if [ "$1" == "--no-container" ]; then
    shift
fi

echo "Building $APP_NAME AppImage v$VERSION for $ARCH..."

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
elif [ ! -f "packaging/appimage/kuvpn.png" ] || [ ! -s "packaging/appimage/kuvpn.png" ]; then
    if command -v convert >/dev/null 2>&1; then
        # Create a burgundy placeholder
        convert -size 256x256 xc:"#800020" packaging/appimage/kuvpn.png
    else
        # Last resort: if we are in the container, we might have librsvg2-dev but maybe not rsvg-convert binary?
        # Actually ubuntu focal has rsvg-convert in librsvg2-bin
        # Let's just touch it if all else fails, but linuxdeploy needs a real PNG.
        # We'll use a small base64 encoded 1x1 red dot png as absolute fallback
        echo "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==" | base64 -d > packaging/appimage/kuvpn.png
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
# Ensure we include appindicator for the tray and other libs often missing in minimal setups
# libnss3 and others are often needed by headless_chrome/chromium
# We add libayatana-ido3 and libdbusmenu-glib to fix "undefined symbol: g_once_init_leave_pointer"
export EXTRA_LIBS="libayatana-appindicator3.so.1;libappindicator3.so.1;libayatana-ido3-0.4.so.0;libdbusmenu-glib.so.4;libdbusmenu-gtk3.so.4;libpangoft2-1.0.so.0;libnss3.so;libnssutil3.so;libsmime3.so;libnspr4.so;libatk-1.0.so.0;libatk-bridge-2.0.so.0;libcups.so.2;libgbm.so.1"

# Make sure linuxdeploy can find the gtk plugin
cp packaging/appimage/linuxdeploy-plugin-gtk.sh ./linuxdeploy-plugin-gtk.sh

./packaging/appimage/linuxdeploy --appimage-extract-and-run --appdir "$APPDIR" \
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
