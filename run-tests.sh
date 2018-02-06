#!/bin/bash

set -ev

(cd src/core/ && cargo test)

pwd
