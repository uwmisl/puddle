#!/bin/bash

set -ev

# test the core
pushd src/core/
cargo test
# check for formatting
cargo fmt -- --write-mode diff
popd

# test the python bindings
pushd src/python/
pipenv install --dev
pipenv run pytest
popd
