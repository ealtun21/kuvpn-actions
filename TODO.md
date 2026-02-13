# TODO

## KUVPN GUI

- Font is not working, tested in appimage, it's not actually reading the font, using the system font of that name, Needs fixing (DONE)

- Update code base for windows support, 
    - Getting DSID works (DONE)
    - No need to ship chromium for windows, as it works fine with edge, which is shipped by default. (DONE)
    - Remove the need to for escelation for running openconnect ( sudo, doas, etc. ), for windows, we want to use run openconnect as admin, may need extra dependcncies, we'll add them only for windows (DONE)
    - App open in a cmd, add build option for windows to avoid that. (DONE)
    - Current installers ships openconnect with the binary after installer, however the code isn't able to test it, may be releated to path for windows, or commands being nix, thisd will be needed to look into. (DONE)
    - Check if openconnect is getting installed correctly. (DONE - via bundled path check)
    - Sign binary in a way that windows isn't complaining about not knowing what it is (after everything else is done) (PENDING - Requires certificate)
    - Remove building cli for windows, we don't really need a cli, the gui is enough. (DONE)

- Modify appimage builder for linux to put appimages inside dist folder instead of just in source path. (DONE)