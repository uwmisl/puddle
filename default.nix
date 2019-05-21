with import <nixpkgs> {
  overlays = [
    # set up the rust overlay for up-to-date rust versions
    (import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz))
  ];
};
let
  # using rust overlay
  rustChannel = rustChannelOf { channel = "1.32.0"; };
  rust = rustChannel.rust.override {
    extensions = ["rust-src"];
    targets = [
      "x86_64-unknown-linux-gnu"
      "armv7-unknown-linux-musleabihf"
    ];
  };

  # don't install arm stuff on travis
  in_ci = builtins.getEnv "CI" == "true";
  arm = import <nixpkgs> { crossSystem.config = "armv7l-unknown-linux-musleabihf"; };
in
stdenv.mkDerivation {
  name = "puddle";
  buildInputs = [
    (if in_ci then null else arm.stdenv.cc)
    rust
    rustracer
    python37Packages.flake8
  ];
  RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/src";
}
