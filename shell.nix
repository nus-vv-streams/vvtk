with (import <nixpkgs> {});
stdenv.mkDerivation {
  name = "rust-env";
  buildInputs = [
    ffmpeg_4
    libiconvReal
    pkg-config
    pkgs.darwin.apple_sdk.frameworks.Security
    pkgs.darwin.apple_sdk.frameworks.ApplicationServices
    pkgs.darwin.apple_sdk.frameworks.CoreVideo
    pkgs.darwin.apple_sdk.frameworks.AppKit
  ];

  # Set Environment variables
  RUST_BACKTRACE = 1;
}
