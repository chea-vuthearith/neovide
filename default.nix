{ lib
, stdenv
, rustPlatform
, fetchFromGitHub
, pkg-config
, fontconfig
, freetype
, libglvnd
, libxkbcommon
, wayland
, xorg
, python3
, cmake
, neovim
, makeWrapper
, darwin
}:

rustPlatform.buildRustPackage rec {
  pname = "neovide";
  version = "0.15.2";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    python3
    cmake
    makeWrapper
  ];

  buildInputs = [
    fontconfig
    freetype
  ] ++ lib.optionals stdenv.isLinux [
    libglvnd
    libxkbcommon
    wayland
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.AppKit
    darwin.apple_sdk.frameworks.CoreGraphics
    darwin.apple_sdk.frameworks.CoreServices
    darwin.apple_sdk.frameworks.CoreVideo
    darwin.apple_sdk.frameworks.Foundation
    darwin.apple_sdk.frameworks.Metal
    darwin.apple_sdk.frameworks.QuartzCore
  ];

  # Neovide requires runtime access to neovim
  postInstall = ''
    wrapProgram $out/bin/neovide \
      --prefix PATH : ${lib.makeBinPath [ neovim ]}
  '' + lib.optionalString stdenv.isLinux ''
    install -Dm644 assets/neovide.desktop -t $out/share/applications
    for size in 16x16 32x32 48x48 256x256; do
      install -Dm644 assets/neovide-$size.png \
        $out/share/icons/hicolor/$size/apps/neovide.png
    done
  '';

  # Tests require a display server
  doCheck = false;

  postFixup = lib.optionalString stdenv.isLinux ''
    patchelf --add-rpath ${lib.makeLibraryPath [ libglvnd ]} $out/bin/neovide
  '';

  meta = with lib; {
    description = "No Nonsense Neovim Client in Rust";
    homepage = "https://neovide.dev/";
    license = licenses.mit;
    mainProgram = "neovide";
    maintainers = with maintainers; [ ];
    platforms = platforms.unix;
  };
}
