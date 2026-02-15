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
        echo "Using Podman to build aarch64 AppImage (Cross-compilation)..."
        # We DON'T use --platform linux/arm64 here because we want to run x86_64 tools in the container
        podman build --build-arg BUILD_ARGS="$BUILD_ARGS" -t kuvpn-builder-aarch64 -f packaging/appimage/Dockerfile.aarch64 .
        
        CONTAINER_ID=$(podman create kuvpn-builder-aarch64)
        mkdir -p dist
        podman cp "$CONTAINER_ID":/build/dist/${APP_NAME}-minimal-${ARCH}.AppImage dist/
        if [ "$1" == "--full" ]; then
             podman cp "$CONTAINER_ID":/build/dist/${APP_NAME}-full-${ARCH}.AppImage dist/
        fi
        podman rm "$CONTAINER_ID"
        
        echo "Successfully built and extracted aarch64 AppImage(s) to dist/."
        exit 0
    else
        echo "Podman not found or already in container. Building on host..."
    fi
fi

if [ "$1" == "--no-container" ]; then
    shift
fi

echo "Building $APP_NAME AppImage v$VERSION for $ARCH..."

# Build the GUI binary (Cross-compile)
cargo build -p kuvpn-gui --release --target aarch64-unknown-linux-gnu

# Setup AppDir
APPDIR="packaging/appimage/AppDir_aarch64"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp target/aarch64-unknown-linux-gnu/release/kuvpn-gui "$APPDIR/usr/bin/"

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

# We use x86_64 tools to package aarch64 assets because we are on an x86_64 host
# Download x86_64 linuxdeploy if needed
if [ ! -f "packaging/appimage/linuxdeploy" ]; then
    curl -L -o packaging/appimage/linuxdeploy https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
    chmod +x packaging/appimage/linuxdeploy
fi

# Download linuxdeploy-plugin-gtk if needed
if [ ! -f "packaging/appimage/linuxdeploy-plugin-gtk.sh" ]; then
    curl -L -o packaging/appimage/linuxdeploy-plugin-gtk.sh https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh
    chmod +x packaging/appimage/linuxdeploy-plugin-gtk.sh
fi

# Download x86_64 appimagetool if needed
if [ ! -f "packaging/appimage/appimagetool" ]; then
    curl -L -o packaging/appimage/appimagetool https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x packaging/appimage/appimagetool
fi

# Build Minimal AppImage
export NO_STRIP=1
export DEPLOY_GTK_VERSION=3

cp packaging/appimage/linuxdeploy-plugin-gtk.sh ./linuxdeploy-plugin-gtk.sh

# List of libraries to bundle (Targeting aarch64 paths)
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
    libffi.so.8
    libffi.so.7
    libpcre.so.3
    libpcre2-8.so.0
    libuuid.so.1
    libssl.so.3
    libcrypto.so.3
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
    if [ -n "$LIB_PATH" ]; then
        LIBS_TO_BUNDLE="$LIBS_TO_BUNDLE --library $LIB_PATH"
    fi
done

# We use the x86_64 linuxdeploy but we tell it NOT to run the executable 
# because it can't (Exec format error).
# We also need to be careful with the GTK plugin as it might try to run some aarch64 binaries.
./packaging/appimage/linuxdeploy --appimage-extract-and-run --appdir "$APPDIR" \
    --executable "$APPDIR/usr/bin/kuvpn-gui" \
    --desktop-file "$APPDIR/usr/share/applications/kuvpn.desktop" \
    --icon-file packaging/appimage/kuvpn.png \
    $LIBS_TO_BUNDLE \
    --custom-apprun scripts/AppRun.sh
    # We skip --plugin gtk for now if it causes issues with cross-builds, 
    # but let's see if it works. Some parts of it might fail.

    # Manual fixups for aarch64
    if [ -d "$APPDIR/usr/lib/aarch64-linux-gnu" ]; then
        cp -rn "$APPDIR"/usr/lib/aarch64-linux-gnu/* "$APPDIR/usr/lib/" || true
    fi

    mkdir -p "$APPDIR/usr/lib/gio/modules"
    cp -L /usr/lib/aarch64-linux-gnu/gio/modules/*.so "$APPDIR/usr/lib/gio/modules/" || true

    mkdir -p "$APPDIR/usr/share/glib-2.0/schemas"
    cp -L /usr/share/glib-2.0/schemas/*.gschema.xml "$APPDIR/usr/share/glib-2.0/schemas/" || true
    # We might not be able to run glib-compile-schemas for aarch64 if it's the aarch64 version,
    # but the x86_64 version should work on the same schema files.
    glib-compile-schemas "$APPDIR/usr/share/glib-2.0/schemas/" || true

rm ./linuxdeploy-plugin-gtk.sh

# Use x86_64 appimagetool to create the AppImage but set the architecture metadata
mkdir -p dist
ARCH=$ARCH ./packaging/appimage/appimagetool --appimage-extract-and-run "$APPDIR" "dist/${APP_NAME}-minimal-${ARCH}.AppImage"

# Build Full AppImage (Optional)
if [ "$1" == "--full" ]; then
    echo "Downloading Chromium for full AppImage (aarch64)..."
    # Note: Google doesn't always provide snapshots for Linux_Arm64 in the same way as x64
    REVISION=$(curl -s "https://www.googleapis.com/download/storage/v1/b/chromium-browser-snapshots/o/Linux_Arm64%2FLAST_CHANGE?alt=media")
    if [ -n "$REVISION" ]; then
        curl -L -o packaging/chromium.zip "https://www.googleapis.com/download/storage/v1/b/chromium-browser-snapshots/o/Linux_Arm64%2F${REVISION}%2Fchrome-linux.zip?alt=media"
        mkdir -p "$APPDIR/usr/lib/chromium"
        unzip -q packaging/chromium.zip -d packaging/appimage/AppDir_aarch64/usr/lib/
        mv "$APPDIR/usr/lib/chrome-linux/"* "$APPDIR/usr/lib/chromium/"
        rm -rf "$APPDIR/usr/lib/chrome-linux" packaging/chromium.zip
    else
        echo "Warning: Could not find Chromium aarch64 snapshot. Skipping full build part."
    fi
    
    ARCH=$ARCH ./packaging/appimage/appimagetool --appimage-extract-and-run "$APPDIR" "dist/${APP_NAME}-full-${ARCH}.AppImage"
fi

echo "Done!"