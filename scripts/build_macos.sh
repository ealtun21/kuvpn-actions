#!/bin/bash
set -e

APP_NAME="KUVPN"
VERSION="2.0.2"

echo "Building $APP_NAME for macOS..."

# Build the binaries
cargo build -p kuvpn-cli --release
cargo build -p kuvpn-gui --release

# Setup App Bundle
APP_BUNDLE="${APP_NAME}.app"
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

cp target/release/kuvpn-gui "$APP_BUNDLE/Contents/MacOS/"
cp crates/kuvpn-gui/assets/ku.svg "$APP_BUNDLE/Contents/Resources/"

# Create Info.plist
cat > "$APP_BUNDLE/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>kuvpn-gui</string>
    <key>CFBundleIdentifier</key>
    <string>tr.edu.ku.kuvpn</string>
    <key>CFBundleName</key>
    <string>KUVPN</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>ku.svg</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
</dict>
</plist>
EOF

if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Creating DMG..."
    hdiutil create -volname "${APP_NAME}" -srcfolder "${APP_BUNDLE}" -ov -format UDZO "${APP_NAME}.dmg"
    
    echo "Creating PKG..."
    pkgbuild --component "${APP_BUNDLE}" --install-location /Applications "${APP_NAME}.pkg"
    
    echo "Done! ${APP_NAME}.dmg and ${APP_NAME}.pkg created."
else
    echo "Not on macOS, skipping DMG/PKG creation. App bundle is at ${APP_BUNDLE}"
fi
