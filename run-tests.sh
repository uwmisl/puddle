#!/bin/bash

set -ev

(cd src/core/ && cargo test)
(cd src/python/ && pipenv install --dev && pipenv run pytest)
