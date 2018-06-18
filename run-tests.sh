#!/bin/bash

# -e terminate the script on errors
set -e

# deny all warnings, converting them to errors
if [ $CI == "true" ]; then
    echo "Denying Rust warnings..."
    export RUSTFLAGS="-D warnings"
else
    echo "Allowing Rust warnings..."
fi

# -v print every line before running it
set -v

# test the core
pushd src/core/

# check for formatting
cargo fmt -- --write-mode diff

# check for clippy lints
cargo +nightly clippy

# try the regular build and tests
cargo build
cargo test

# just check the things that require the pi
cargo check --features pi
cargo check --tests --features pi

popd


# test the python bindings
pushd src/python/

# install the virtual environment
pipenv install --dev --python python3

# check for style
pipenv run flake8

# run the tests
pipenv run pytest

popd
