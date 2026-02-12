#!/bin/bash
set -e

# Configuration
APP_NAME="KUVPN"
VERSION="2.0.2"
ARCH="aarch64"

# Podman/Docker detection and wrapper
if [ "$1" != "--no-container" ] && [ ! -f /.containerenv ] && [ ! -f /run/.containerenv ]; then
    BUILD_ARGS=""
    if [ "$1" == "--full" ]; then
        BUILD_ARGS="--full"
    fi

    if command -v podman >/dev/null 2>&1; then
        echo "Using Podman to build aarch64 AppImage..."
        podman build --platform linux/arm64 --build-arg BUILD_ARGS="$BUILD_ARGS" -t kuvpn-builder-aarch64 -f packaging/appimage/Dockerfile.aarch64 .
        
        CONTAINER_ID=$(podman create kuvpn-builder-aarch64)
        podman cp "$CONTAINER_ID":/build/${APP_NAME}-minimal-${ARCH}.AppImage .
        if [ "$1" == "--full" ]; then
             podman cp "$CONTAINER_ID":/build/${APP_NAME}-full-${ARCH}.AppImage .
        fi
        podman rm "$CONTAINER_ID"
        
        echo "Successfully built and extracted aarch64 AppImage(s)."
        exit 0
    else
        echo "Podman not found or already in container. Building on host..."
    fi
fi

if [ "$1" == "--no-container" ]; then
    shift
fi

echo "Building $APP_NAME AppImage v$VERSION for $ARCH..."

# Build the GUI binary
cargo build -p kuvpn-gui --release

# Setup AppDir
APPDIR="packaging/appimage/AppDir_aarch64"
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

# Create icon
if [ -f "crates/kuvpn-gui/assets/ku.svg" ] && command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w 256 -h 256 "crates/kuvpn-gui/assets/ku.svg" -o packaging/appimage/kuvpn.png
fi

# Download linuxdeploy aarch64 if needed
if [ ! -f "packaging/appimage/linuxdeploy-aarch64" ]; then
    curl -L -o packaging/appimage/linuxdeploy-aarch64 https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-aarch64.AppImage
    chmod +x packaging/appimage/linuxdeploy-aarch64
fi

# Download linuxdeploy-plugin-gtk if needed
if [ ! -f "packaging/appimage/linuxdeploy-plugin-gtk.sh" ]; then
    curl -L -o packaging/appimage/linuxdeploy-plugin-gtk.sh https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh
    chmod +x packaging/appimage/linuxdeploy-plugin-gtk.sh
fi

# Download appimagetool aarch64 if needed
if [ ! -f "packaging/appimage/appimagetool-aarch64" ]; then
    curl -L -o packaging/appimage/appimagetool-aarch64 https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-aarch64.AppImage
    chmod +x packaging/appimage/appimagetool-aarch64
fi

# Build Minimal AppImage
export NO_STRIP=1
export DEPLOY_GTK_VERSION=3

cp packaging/appimage/linuxdeploy-plugin-gtk.sh ./linuxdeploy-plugin-gtk.sh

# List of libraries to bundle (adjusted for aarch64 paths if needed, but the loop finds them)
LIBS_TO_BUNDLE_LIST="
    libayatana-appindicator3.so.1 
    libayatana-ido3-0.4.so.0 
    libayatana-indicator3.so.7 
    libdbusmenu-glib.so.4 
    libdbusmenu-gtk3.so.4 
    libindicator3.so.7 
    libpangoft2-1.0.so.0 
    libnss3.so 
    libnssutil3.so 
    libsmime3.so 
    libnspr4.so 
    libatk-1.0.so.0 
    libatk-bridge-2.0.so.0 
    libcups.so.2 
    libgbm.so.1 
    libdrm.so.2
    libxdo.so.3 
    libglib-2.0.so.0 
    libgio-2.0.so.0 
    libgobject-2.0.so.0 
    libgmodule-2.0.so.0
    libdbus-1.so.3
    libproxy.so.1
    libfreetype.so.6
    libfontconfig.so.1
    libharfbuzz.so.0
    libfribidi.so.0
    libgraphite2.so.3
    libexpat.so.1
    libz.so.1
    libpng16.so.16
    libjpeg.so.8
    libwayland-client.so.0
    libwayland-cursor.so.0
    libwayland-egl.so.1
    libxkbcommon.so.0
    liblzma.so.5
    liblz4.so.1
    libgcrypt.so.20
    libgpg-error.so.0
    libblkid.so.1
    libmount.so.1
    libselinux.so.1
    libffi.so.7
    libpcre.so.3
    libuuid.so.1
    libssl.so.1.1
    libcrypto.so.1.1
    libasound.so.2
    libpulse.so.0
    libpulse-mainloop-glib.so.0
    libsqlite3.so.0
    libxml2.so.2
    libstdc++.so.6
    libgcc_s.so.1
    libxkbcommon-x11.so.0
    libxcb-xkb.so.1
    libX11-xcb.so.1
    libxcb-render.so.0
    libxcb-shm.so.0
    libxcb-util.so.1
"

LIBS_TO_BUNDLE=""
for lib in $LIBS_TO_BUNDLE_LIST; do
    LIB_PATH=$(find /usr/lib/aarch64-linux-gnu -name "$lib" | head -n 1)
    if [ -z "$LIB_PATH" ]; then
        LIB_PATH=$(find /usr/lib -name "$lib" | head -n 1)
    fi
    if [ -n "$LIB_PATH" ]; then
        LIBS_TO_BUNDLE="$LIBS_TO_BUNDLE --library $LIB_PATH"
    fi
done

./packaging/appimage/linuxdeploy-aarch64 --appimage-extract-and-run --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/kuvpn-gui" \
    --desktop-file "$APPDIR/usr/share/applications/kuvpn.desktop" \
    --icon-file packaging/appimage/kuvpn.png \
    $LIBS_TO_BUNDLE \
    --plugin gtk \
    --custom-apprun scripts/AppRun.sh
 
    if [ -d "$APPDIR/usr/lib/aarch64-linux-gnu" ]; then
        cp -rn "$APPDIR"/usr/lib/aarch64-linux-gnu/* "$APPDIR/usr/lib/" || true
    fi

    mkdir -p "$APPDIR/usr/lib/gio/modules"
    cp -L /usr/lib/aarch64-linux-gnu/gio/modules/*.so "$APPDIR/usr/lib/gio/modules/" || true

    mkdir -p "$APPDIR/usr/share/glib-2.0/schemas"
    cp -L /usr/share/glib-2.0/schemas/*.gschema.xml "$APPDIR/usr/share/glib-2.0/schemas/" || true
    glib-compile-schemas "$APPDIR/usr/share/glib-2.0/schemas/" || true

rm ./linuxdeploy-plugin-gtk.sh

ARCH=$ARCH ./packaging/appimage/appimagetool-aarch64 --appimage-extract-and-run "$APPDIR" "${APP_NAME}-minimal-${ARCH}.AppImage"

echo "Done!"
