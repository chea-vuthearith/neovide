{
  description = "Neovide: No Nonsense Neovim Client in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-analyzer"];
        };

        buildInputs = with pkgs;
          [
            fontconfig
            freetype
          ]
          ++ lib.optionals stdenv.isLinux [
            libglvnd
            libxkbcommon
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
          ]
          ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.AppKit
            darwin.apple_sdk.frameworks.CoreGraphics
            darwin.apple_sdk.frameworks.CoreServices
            darwin.apple_sdk.frameworks.CoreVideo
            darwin.apple_sdk.frameworks.Foundation
            darwin.apple_sdk.frameworks.Metal
            darwin.apple_sdk.frameworks.QuartzCore
          ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          python3
          cmake
          makeWrapper
        ];
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "neovide";
          version = "0.15.2";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit buildInputs nativeBuildInputs;

          # Neovide requires runtime access to neovim
          postInstall =
            ''
              wrapProgram $out/bin/neovide \
                --prefix PATH : ${pkgs.lib.makeBinPath [pkgs.neovim]}
            ''
            + pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              install -Dm644 assets/neovide.desktop -t $out/share/applications
              for size in 16x16 32x32 48x48 256x256; do
                install -Dm644 assets/neovide-$size.png \
                  $out/share/icons/hicolor/$size/apps/neovide.png
              done
            '';

          # Tests require a display server
          doCheck = false;

          postFixup = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            patchelf --add-rpath ${pkgs.lib.makeLibraryPath [pkgs.libglvnd]} $out/bin/neovide
          '';

          meta = with pkgs.lib; {
            description = "No Nonsense Neovim Client in Rust";
            homepage = "https://neovide.dev/";
            license = licenses.mit;
            mainProgram = "neovide";
            platforms = platforms.unix;
          };
        };

        packages.neovide = self.packages.${system}.default;

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/neovide";
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          nativeBuildInputs =
            nativeBuildInputs
            ++ [
              rustToolchain
            ];

          shellHook = ''
            echo "Neovide development environment"
            echo "Rust version: $(rustc --version)"
          '';

          # Set library path for running during development
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };
      }
    );
}
