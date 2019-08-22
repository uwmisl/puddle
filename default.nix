with import <nixpkgs> {
  overlays = [
    # set up the rust overlay for up-to-date rust versions
    (import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz))
  ];
};
let
  # don't install arm stuff on travis
  ci = builtins.getEnv "CI" != "";
  not_ci = x: if ci then null else x;
  filter = builtins.filter (x: !isNull x);

  # using rust overlay
  rustChannel = rustChannelOf { channel = "1.36.0"; };
  rust = rustChannel.rust.override {
    extensions = if ci then [] else [ "rust-src" "rust-analysis" "rls-preview" ];
    targets = filter [
      "wasm32-unknown-unknown"
      (not_ci "armv7-unknown-linux-musleabihf")
    ];
  };

  arm = import <nixpkgs> { crossSystem.config = "armv7l-unknown-linux-musleabihf"; };
  unstable = import <nixpkgs-unstable> {};
in
stdenv.mkDerivation {
  name = "puddle";
  buildInputs = filter [
    nodejs
    unstable.wasm-pack

    (not_ci arm.stdenv.cc)
    rust

    python37Packages.pyflakes
    python37Packages.yapf
    python37Packages.requests
  ];
  RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/src";
}
