
PROFILE ?= release
ifeq (${PROFILE},release)
  PROFILE_FLAGS=--release
else ifeq (${PROFILE},debug)
  PROFILE_FLAGS=
else
  $(error Invalid profile: ${PROFILE})
endif

RSYNC ?= rsync -iP --compress-level=9
TARGET ?= armv7-unknown-linux-musleabihf
PI ?= blueberry-pie.zt

define i
@echo
@echo -e
endef

.PHONY: all
all: test

.PHONY: test
test: test-rust test-python

.PHONY: test-python
test-python:
	$i "Checking version..."
	cd src/python; ./setup.py --version
	$i "Linting src..."
	cd src/python; pyflakes puddle
	$i "Linting tests..."
	cd src/python; pyflakes tests
	$i "Linting examples..."
	cd src/python; pyflakes examples
	$i "Testing..."
	cd src/python; time ./setup.py test
	$i "Checking formatting..."
	cd src/python; yapf --recursive --diff .

.PHONY: test-rust
test-rust:
	$i "Checking version..."
	rustc --version
	cargo --version
	$i "Building..."
	cd src; time cargo build
	$i "Testing..."
	cd src; time cargo test
	$i "Linting..."
	cd src; time cargo clippy --tests

.PHONY: sync
sync: sync-boards sync-server sync-pi-test

.PHONY: sync-boards
sync-boards:
	${RSYNC} --relative tests/./arches/*.yaml ${PI}:

.PHONY: sync-server
sync-server:
	cd src; cargo build ${PROFILE_FLAGS} --target ${TARGET} --bin puddle-server
	${RSYNC} src/target/${TARGET}/${PROFILE}/puddle-server ${PI}:

.PHONY: sync-pi-test
sync-pi-test:
	cd src; cargo build --target ${TARGET} --bin pi-test
	${RSYNC} src/target/${TARGET}/debug/pi-test ${PI}:
