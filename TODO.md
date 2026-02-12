# TODO

## KUVPN GUI

- Fix scripts/build_appimage.sh to actually contain all dependencies required to run, currently lacks some, resulting in it working on dev env, but failing on VM-based envirments. 
    - Expansion: Consider using a Podman-based build environment (like cargo-zigbuild or a dedicated Ubuntu 20.04 container) to ensure the AppImage is built against older GLIBC versions for maximum compatibility.
    - Alternative, bind with musl, inside appimage, may be more diffucult to do but, even better compatilbilty.

- Add persistance to GUI settings, with a reset to defaults button

- Make the cancel while connecting operation also be able to cancel/interrup the process of asking for escalation password. 
     - You need to wrap that call in a thread that can be dropped or a process that can be killed. If the user hits "Cancel," the GUI should stop waiting on the stdin of the password prompt.

- Fix the bug, where if cancel and connect button is spammed, the browser gets lauched but not terminated resulting in the idle state of the program while it thinks it's connecting. 

- Add Windows Support, involves making an installer that can install both the application, and openconnect too, since unlike linux on windows it's much harder to install, we just bundle a way to install that. 
    - Diffiucult, may be in backlog till other are done.
    - Windows: Bundle openconnect.exe and its dependencies (like wintun.dll) using NSIS or Inno Setup. Since OpenConnect on Windows requires a virtual network adapter, the installer will likely need admin privileges.
    - We are gonna need to add window-only coded under a compile flag as this requires a good amount of work.

- Generate a macos .pkg file or .dmg file for macos, no need to ship openconnect, but if we can, great, openconnect can be installed via brew.

- Add warning if openconnect can not be found to GUI ui, also add test button on the openconnect-path setting, to make sure it actually works, if it doesn't work, add the same warning. The can not be found should be testing the OC-path and also any other methods of finding openconnect we have built-in.

- Refactor GUI into file, oversized main.rs is hard to maintain.

