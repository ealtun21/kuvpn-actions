# TODO

## KUVPN GUI

- Fix scripts/build_appimage.sh to actually contain all dependencies required to run, currently lacks some, resulting in it working on dev env, but failing on VM-based envirments. (DONE: Added more EXTRA_LIBS and Dockerfile for compatibility)

- Add persistance to GUI settings, with a reset to defaults button (DONE)

- Make the cancel while connecting operation also be able to cancel/interrup the process of asking for escalation password. (DONE)

- Fix the bug, where if cancel and connect button is spammed, the browser gets lauched but not terminated resulting in the idle state of the program while it thinks it's connecting. (DONE)

- Add Windows Support, involves making an installer that can install both the application, and openconnect too, since unlike linux on windows it's much harder to install, we just bundle a way to install that. (IN PROGRESS: Groundwork done, cross-platform code guards added)
    - Diffiucult, may be in backlog till other are done.
    - Windows: Bundle openconnect.exe and its dependencies (like wintun.dll) using NSIS or Inno Setup. Since OpenConnect on Windows requires a virtual network adapter, the installer will likely need admin privileges.

- Generate a macos .pkg file or .dmg file for macos, no need to ship openconnect, but if we can, great, openconnect can be installed via brew. (IN PROGRESS: Updated CI to generate .app bundle)

- Add warning if openconnect can not be found to GUI ui, also add test button on the openconnect-path setting, to make sure it actually works, if it doesn't work, add the same warning. (DONE)

- Refactor GUI into file, oversized main.rs is hard to maintain. (DONE)