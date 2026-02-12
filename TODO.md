# TODO

## KUVPN GUI

- Add Windows Support, involves making an installer that can install both the application, and openconnect too, since unlike linux on windows it's much harder to install, we just bundle a way to install that. (IN PROGRESS: Groundwork done, cross-platform code guards added, installer script and CI workflow implemented)
    - Diffiucult, may be in backlog till other are done.
    - Windows: Bundle openconnect.exe and its dependencies (like wintun.dll) using NSIS or Inno Setup. Since OpenConnect on Windows requires a virtual network adapter, the installer will likely need admin privileges.

- Add warning if openconnect can not be found to GUI ui, telling user to install or select the path from the config and OC (DONE)

- Font is not working, tested in appimage, it's not actually reading the font, using the system font of that name, Needs fixing (IN PROGRESS: Attempted fix by using internal font name 'JetBrainsMono NFM')

- Refactor GUI into file, oversized main.rs is hard to maintain. (DONE)

- Make another script to build aarch64 appimages, another script. (DONE)

- Crosscompile to macos ( not sure how possible, may need to use github workflow instead)