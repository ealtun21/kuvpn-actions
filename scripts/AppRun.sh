#!/bin/bash
export SELF=$(readlink -f "$0")
export HERE="${SELF%/*}"

# Prioritize bundled libs
export LD_LIBRARY_PATH="${HERE}/usr/lib:${HERE}/usr/lib/x86_64-linux-gnu:${HERE}/lib/x86_64-linux-gnu:${HERE}/lib:${LD_LIBRARY_PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
export GIO_MODULE_DIR="${HERE}/usr/lib/gio/modules"
export GSETTINGS_SCHEMA_DIR="${HERE}/usr/share/glib-2.0/schemas"

# Disable host GTK modules and paths to avoid loading incompatible host-system plugins
export GTK_MODULES=""
export GTK_PATH=""

# Force use of bundled glib/gobject/gio to avoid host symbol mismatches 
# when dlopening appindicator or other plugins.
# Note: We specifically EXCLUDE libX11 and libxcb to avoid conflicts with host drivers.
PRELOAD_LIBS=""
for lib in libglib-2.0.so.0 libgobject-2.0.so.0 libgio-2.0.so.0 libgmodule-2.0.so.0 libdbus-1.so.3 libproxy.so.1 libstdc++.so.6 libgcc_s.so.1 libdbusmenu-gtk3.so.4 libdbusmenu-glib.so.4 libfontconfig.so.1 libfreetype.so.6 libatk-1.0.so.0 libatk-bridge-2.0.so.0 libxkbcommon-x11.so.0 libxcb-xkb.so.1 libX11-xcb.so.1; do
    FOUND=""
    for dir in "${HERE}/usr/lib" "${HERE}/usr/lib/x86_64-linux-gnu" "${HERE}/lib" "${HERE}/lib/x86_64-linux-gnu"; do
        if [ -f "$dir/$lib" ]; then
            FOUND="$dir/$lib"
            break
        fi
    done

    if [ -n "$FOUND" ]; then
        if [ -z "$PRELOAD_LIBS" ]; then
            PRELOAD_LIBS="$FOUND"
        else
            PRELOAD_LIBS="$PRELOAD_LIBS:$FOUND"
        fi
    fi
done

if [ -n "$PRELOAD_LIBS" ]; then
    export LD_PRELOAD="$PRELOAD_LIBS${LD_PRELOAD:+:$LD_PRELOAD}"
fi

# Chrome path for full version
export KUVPN_CHROME_PATH="${HERE}/usr/lib/chromium/chrome"
if [ ! -f "$KUVPN_CHROME_PATH" ]; then
    unset KUVPN_CHROME_PATH
fi

# Run the app
exec "${HERE}/usr/bin/kuvpn-gui" "$@"