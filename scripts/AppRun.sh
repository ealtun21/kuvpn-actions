#!/bin/bash
export SELF=$(readlink -f "$0")
export HERE="${SELF%/*}"

# Prioritize bundled libs
export LD_LIBRARY_PATH="${HERE}/usr/lib:${HERE}/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"

# Force use of bundled glib/gobject to avoid host symbol mismatches 
# when dlopening appindicator
if [ -f "${HERE}/usr/lib/libglib-2.0.so.0" ]; then
    export LD_PRELOAD="${HERE}/usr/lib/libglib-2.0.so.0:${HERE}/usr/lib/libgobject-2.0.so.0:${HERE}/usr/lib/libgio-2.0.so.0"
fi

# Chrome path for full version
export KUVPN_CHROME_PATH="${HERE}/usr/lib/chromium/chrome"
if [ ! -f "$KUVPN_CHROME_PATH" ]; then
    unset KUVPN_CHROME_PATH
fi

# Run the app
exec "${HERE}/usr/bin/kuvpn-gui" "$@"