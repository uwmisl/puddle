#!/bin/bash

set -ev

# test the core
(cd src/core/ && cargo test)

# test the python bindings
(cd src/python/ && pipenv install --dev && pipenv run pytest)
