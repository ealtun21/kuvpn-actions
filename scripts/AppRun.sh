#!/bin/bash
export SELF=$(readlink -f "$0")
export HERE="${SELF%/*}"

export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${HERE}/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"

# Chrome path for full version
export KUVPN_CHROME_PATH="${HERE}/usr/lib/chromium/chrome"
if [ ! -f "$KUVPN_CHROME_PATH" ]; then
    unset KUVPN_CHROME_PATH
fi

# Run the app
exec "${HERE}/usr/bin/kuvpn-gui" "$@"
