with import <nixpkgs> {
  overlays = [
    # set up the rust overlay for up-to-date rust versions
    (import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz))
  ];
};
let
  # don't install arm stuff on travis
  not_ci = x: if builtins.getEnv "CI" == "true" then null else x;

  # using rust overlay
  rustChannel = rustChannelOf { channel = "1.34.2"; };
  rust = rustChannel.rust.override {
    extensions = [(not_ci "rust-src")];
    targets = [
      "x86_64-unknown-linux-gnu"
      (not_ci "armv7-unknown-linux-musleabihf")
    ];
  };

  arm = import <nixpkgs> { crossSystem.config = "armv7l-unknown-linux-musleabihf"; };
in
stdenv.mkDerivation {
  name = "puddle";
  buildInputs = [
    (not_ci arm.stdenv.cc)
    rust
    (not_ci rustracer)
    python37Packages.yapf
    python37Packages.requests
  ];
  RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/src";
}
