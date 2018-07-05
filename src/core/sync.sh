#!/bin/bash

set -ev

pi=bananapi

rsync='rsync --compress --times --progress'

# compile
cross build --target armv7-unknown-linux-gnueabihf --features vision,pi

# kill any running servers
ssh $pi -- killall -q puddle-server || true

# sync the binaries
$rsync target/armv7-unknown-linux-gnueabihf/debug/{vision-test,puddle-server,pi-test} $pi:

# sync the python
$rsync ../python/puddle.py $pi:puddle/src/python/
$rsync ../python/examples/*.py $pi:puddle/src/python/examples/
