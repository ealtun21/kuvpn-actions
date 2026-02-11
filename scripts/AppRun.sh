#!/bin/bash
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export KUVPN_CHROME_PATH="${HERE}/usr/lib/chromium/chrome"

if [ ! -f "$KUVPN_CHROME_PATH" ]; then
    unset KUVPN_CHROME_PATH
fi

exec kuvpn-gui "$@"
