#!/usr/bin/env bash

set -ev

pi=blueberry-pie.zt

rsync='rsync -iP --compress-level=9'

target='armv7-unknown-linux-musleabihf'

# compile
cargo build --target $target --features pi

# kill any running servers
# ssh $pi -- killall -q puddle-server || true

# sync the binaries
# $rsync target/$target/debug/{puddle-server,pi-test} $pi:
$rsync target/$target/debug/{puddle-server,pi-test} $pi:

# sync the python
# $rsync ../python/puddle.py $pi:puddle/src/python/
# $rsync ../python/examples/*.py $pi:puddle/src/python/examples/

# sync the boards
$rsync --relative ../../tests/./arches/*.json $pi:
