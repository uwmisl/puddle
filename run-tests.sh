#!/bin/bash

# -e terminate the script on errors
# -v print every line before running it
set -ev

# speed up tests by reducing the step delay
export PUDDLE_STEP_DELAY_MS=5


# test the core
pushd src/core/

# deny all warnings, converting them to errors
export RUSTFLAGS="-D warnings"

# try the regular build
cargo build

# run the tests
cargo test

# check for formatting
cargo fmt -- --write-mode diff

popd


# test the python bindings
pushd src/python/

# install the virtual environment
pipenv install --dev

# check for style
pipenv run flake8

# run the tests
pipenv run pytest

popd
