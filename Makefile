
PROFILE ?= release
ifeq (${PROFILE},release)
  PROFILE_FLAGS=--release
else ifeq (${PROFILE},debug)
  PROFILE_FLAGS=
else
  $(error Invalid profile: ${PROFILE})
endif

RSYNC ?= rsync -riP --compress-level=9
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
	cd puddle-python; ./setup.py --version
	$i "Linting src..."
	cd puddle-python; pyflakes puddle
	$i "Linting tests..."
	cd puddle-python; pyflakes tests
	$i "Linting examples..."
	cd puddle-python; pyflakes examples
	$i "Testing..."
	cd puddle-python; time ./setup.py test
	$i "Checking formatting..."
	cd puddle-python; yapf --recursive --diff .

.PHONY: test-rust
test-rust:
	$i "Checking version..."
	rustc --version
	cargo --version
	$i "Building..."
	time cargo build
	$i "Testing..."
	time cargo test
	$i "Linting..."
	time cargo clippy --tests
	$i "Formatting..."
	cargo fmt -- --check

.PHONY: test-js
test-js:
	$i "Checking version..."
	npm --version
	$i "Building..."
	cd puddle-js; npm run build -- --mode development

.PHONY: sync
sync: sync-boards sync-server sync-pi-test

.PHONY: sync-boards
sync-boards:
	${RSYNC} --relative tests/./arches/*.yaml ${PI}:

.PHONY: sync-server
sync-server:
	cargo build ${PROFILE_FLAGS} --target ${TARGET} --bin puddle-server
	${RSYNC} target/${TARGET}/${PROFILE}/puddle-server ${PI}:

.PHONY: sync-pi-test
sync-pi-test:
	cargo build --target ${TARGET} --bin pi-test
	${RSYNC} target/${TARGET}/debug/pi-test ${PI}:

.PHONY: sync-web-demo
sync-web-demo:
	cd puddle-js; npm run build
	${RSYNC} puddle-js/dist/ mwillsey.com:/var/www/stuff/puddle-demo/
