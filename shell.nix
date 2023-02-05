with (import <nixpkgs> {});
stdenv.mkDerivation {
  name = "rust-env";
  buildInputs = [
    ffmpeg
    libiconvReal
    pkg-config
  ];

  # Set Environment variables
  RUST_BACKTRACE = 1;
}
