# TODO

## KUVPN GUI

- Font is not working, tested in appimage, it's not actually reading the font, using the system font of that name, Needs fixing, broken on windows, some random icons are showing up that are complety differnt than linux. 

- cleanup kuvpn-gui's openconnect handling code, as on windows builds it has no idea when it's connecting due to no child being passing in lib, we gonna have to do something about how it understands, may require some extra functions on lib to handle that. 

- Cleanup browser handling,

## KUVPN LIB - REFACTORING

- [x] Create a `VpnSession` manager in the library to unify connection logic.
- [x] Implement robust connection state tracking (Connected/Disconnected/Connecting) in the library.
- [x] Remove log-based state detection.
- [x] Improve OpenConnect process management on Windows.
- [x] Ensure browser is strictly closed before starting OpenConnect.
- [x] Refactor `run_login_and_get_dsid` to be more robust and less reliant on generic retries.

# KUVPN LIB

- cleanup kuvpn/src/openconnect.rs, and modify the rest of the crates to handle it, as openconnect is now workin on window build.