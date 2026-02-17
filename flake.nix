{
  description = "KUVPN - Koç University VPN Client";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    nix-appimage.url = "github:ralismark/nix-appimage";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, nix-appimage, ... }:
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
          openssl
          dbus
          glib
          gtk3
          libappindicator-gtk3
          libayatana-appindicator
          libcanberra-gtk3
          xapp
          libX11
          libxcb
          libXdmcp
          libXtst
          libXinerama
          libxkbfile
          libxkbcommon
          librsvg
          xdotool
          gsettings-desktop-schemas
          hicolor-icon-theme
          # wgpu backend requirements
          vulkan-loader
          wayland
        ];

        kuvpnGui = rustPlatform.buildRustPackage {
          pname = "kuvpn-gui";
          version = "2.0.3";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkg-config pkgs.copyDesktopItems pkgs.wrapGAppsHook3 ];
          buildInputs = commonBuildInputs;

          # Runtime library paths for dlopen'd libraries.
          # tray-icon loads libayatana-appindicator3 via dlopen at runtime.
          # Vulkan/wgpu: Do NOT set VK_ICD_FILENAMES here — the AppImage
          # bind-mounts the host's /usr, so the host's Vulkan ICDs are found
          # automatically by the loader's default search paths.
          preFixup = ''
            gappsWrapperArgs+=(
              --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [
                pkgs.libappindicator-gtk3
                pkgs.libayatana-appindicator
                pkgs.vulkan-loader
                pkgs.wayland
                pkgs.libxkbcommon
              ]}"
            )
          '';

          # Install desktop file and icon
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

        mkAppImage = nix-appimage.lib.${system}.mkAppImage;

        # Wrapper script for the AppImage entry point.
        # nix-appimage bind-mounts the host's entire / except /nix, so
        # the host's Vulkan ICD files (e.g. /usr/share/vulkan/icd.d/)
        # are accessible. We must ensure no Nix wrapper has overridden
        # VK_ICD_FILENAMES to point only at Nix store paths, which would
        # hide the host GPU drivers from the Vulkan loader.
        appimageWrapper = pkgs.writeShellScript "kuvpn-gui-appimage" ''
          # Clear any Nix-injected Vulkan ICD overrides so the host GPU is used
          unset VK_ICD_FILENAMES
          unset VK_DRIVER_FILES
          exec "${kuvpnGui}/bin/kuvpn-gui" "$@"
        '';

        appimage = mkAppImage {
          program = appimageWrapper;
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
