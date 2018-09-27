#!/bin/bash

# -e terminate the script on errors
set -e

## allow warnings for now
# deny all warnings, converting them to errors
# if [[ $CI == "true" ]]; then
#     echo "Denying Rust warnings..."
#     export RUSTFLAGS="-D warnings"
# else
#     echo "Allowing Rust warnings..."
# fi

# don't commit large files
deleted=$(mktemp)
git ls-files -d > $deleted
for f in $(git ls-files)
do
    if grep -qx $f $deleted
    then
        echo "$f was deleted!"
        continue
    fi
    size="$(stat -c "%s" $f)"
    if (( $size > 1000 * 1000 ))
    then
        echo "$f is too large ($size)"
        exit 1
    fi
done

# -v print every line before running it
set -v

# test the core
pushd src/core/

# check for formatting
cargo fmt -- --check

# try the regular build and tests
cargo build
cargo test

# just check the things that require the pi
cargo check --features pi
cargo check --tests --features pi

# just check the things that require vision
cargo check --features vision
cargo check --tests --features vision

# just check the things that require vision and pi
cargo check --features vision,pi
cargo check --tests --features vision,pi

popd


# test the python bindings
pushd src/python/

# create the virtualenv if it doesn't exist
pipenv --venv || pipenv --python 3
# install the virtual environment
pipenv install --dev

# check for style
pipenv run flake8

# run the tests
pipenv run pytest

popd
