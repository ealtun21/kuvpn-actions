{
  description = "KUVPN - Koç University VPN Client";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = pkgs.rust-bin.stable.latest.default;
          rustc = pkgs.rust-bin.stable.latest.default;
        };

        commonBuildInputs = with pkgs; [
          pkg-config
          openssl
          dbus
          glib
          gtk3
          libappindicator-gtk3
          xorg.libX11
          xorg.libxcb
          xorg.libXdmcp
          xorg.libXtst
          xorg.libXinerama
          xorg.libxkbfile
          libxkbcommon
          librsvg
          xdotool
        ];

        kuvpnGui = rustPlatform.buildRustPackage {
          pname = "kuvpn-gui";
          version = "2.0.3";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkg-config pkgs.copyDesktopItems ];
          buildInputs = commonBuildInputs;

          # Fix for missing libraries at runtime
          postInstall = ''
            mkdir -p $out/share/applications $out/share/icons/hicolor/256x256/apps
            cp packaging/linux/kuvpn.desktop $out/share/applications/
            cp packaging/linux/kuvpn.png $out/share/icons/hicolor/256x256/apps/kuvpn.png
          '';

          meta = with pkgs.lib; {
            description = "Koç University VPN Client GUI";
            homepage = "https://github.com/KUACC-VALAR-HPC-KOC-UNIVERSITY/kuvpn";
            license = licenses.mit;
            platforms = platforms.linux;
            mainProgram = "kuvpn-gui";
          };
        };

        appimage = pkgs.appimageTools.wrapType2 {
          pname = "kuvpn-gui";
          version = "2.0.3";
          src = kuvpnGui;
          
          extraPkgs = pkgs: [
            pkgs.gtk3
            pkgs.gdk-pixbuf
            pkgs.cairo
            pkgs.pango
            pkgs.atk
            pkgs.libappindicator-gtk3
            pkgs.libayatana-appindicator
            pkgs.xdotool
            pkgs.xorg.libX11
            pkgs.xorg.libXtst
            pkgs.libxkbcommon
            pkgs.dbus
            pkgs.openssl
            pkgs.librsvg
          ];
        };
      in
      {
        packages = {
          default = kuvpnGui;
          inherit appimage;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = commonBuildInputs ++ [
            pkgs.rust-bin.stable.latest.default
            pkgs.rust-analyzer
          ];
          
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath commonBuildInputs}";
        };
      }
    );
}
