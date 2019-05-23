#!/usr/bin/env bash

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

set -x

set +x
echo
echo "+-----------------------------------------------+"
echo "|                 Testing rust                  |"
echo "+-----------------------------------------------+"
echo
set -x

pushd src/

cargo --version
rustc --version

cargo build
cargo test
cargo clippy --tests

cargo check --features pi
cargo check --tests --features pi

cargo fmt -- --check

# # just check the things that require vision
# cargo check --features vision
# cargo check --tests --features vision

# # just check the things that require vision and pi
# cargo check --features vision,pi
# cargo check --tests --features vision,pi

popd


set +x
echo
echo "+-----------------------------------------------+"
echo "|                Testing python                 |"
echo "+-----------------------------------------------+"
echo
set -x

pushd src/python/
python --version
./setup.py --version
./setup.py test
yapf --version
yapf --recursive --diff .
popd
