# This makefile is to provide some shortcuts to the programs.mk file.
# Since I want to move as many implementation details out of the programs.mk file as possible.

default: release

DEBUG_OUTPUT := target/i686-unknown-linux-gnu/debug/execve
RELEASE_OUTPUT := target/i686-unknown-linux-gnu/release/execve

TARGET_PROGRAM := ../example_rust/target/i686-unknown-linux-gnu/release/execve

.PHONY: debug release
release: $(RELEASE_OUTPUT)
debug: $(DEBUG_OUTPUT)

$(TARGET_PROGRAM):
	cd ../example_rust && make release

$(DEBUG_OUTPUT): src $(TARGET_PROGRAM)
	cargo build

$(RELEASE_OUTPUT): src $(TARGET_PROGRAM)
	cargo build --release

.PHONY: clean
clean:
	cargo clean
