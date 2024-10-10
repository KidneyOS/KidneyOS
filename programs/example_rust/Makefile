# This makefile is to provide some shortcuts to the programs.mk file.
# Since I want to move as many implementation details out of the programs.mk file as possible.

default: release

DEBUG_OUTPUT := target/i686-unknown-linux-gnu/debug/example_rust
RELEASE_OUTPUT := target/i686-unknown-linux-gnu/release/example_rust

.PHONY: debug release
release: $(RELEASE_OUTPUT)
debug: $(DEBUG_OUTPUT)

$(DEBUG_OUTPUT): src
	cargo build

$(RELEASE_OUTPUT): src
	cargo build --release

.PHONY: clean
clean:
	cargo clean
