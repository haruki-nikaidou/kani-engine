{
  description = "kani-engine dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          # When rust-toolchain.toml changes, set this to pkgs.lib.fakeSha256,
          # run `nix develop` once, then copy the "got:" hash from the error here.
          sha256 = "sha256-zC8E38iDVJ1oPIzCqTk/Ujo9+9kx9dXq7wAwPMpkpg0=";
        };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            mold
            toolchain
          ];

          buildInputs = with pkgs; [
            # Wayland
            wayland
            wayland-protocols
            libxkbcommon

            # X11
            libX11
            libXi
            libXcursor
            libXrandr
            libXinerama

            # Graphics
            libGL
            vulkan-loader
            vulkan-headers

            # Audio
            alsa-lib
            pipewire

            # System
            udev
            fontconfig
            openssl
          ];

          # Point the dynamic linker at Nix-managed libs at runtime
          LD_LIBRARY_PATH =
            with pkgs;
            lib.makeLibraryPath [
              wayland
              libxkbcommon
              libGL
              vulkan-loader
              alsa-lib
              udev
            ];

          RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            echo "kani-engine dev shell ready"
          '';
        };
      }
    );
}
