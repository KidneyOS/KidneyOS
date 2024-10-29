PROJECT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

# Stubs are explicitly debug so they don't get inlined.
SYSCALL_LIB := $(PROJECT_DIR)build/target/i686-unknown-linux-gnu/debug/libkidneyos_syscalls.rlib

$(SYSCALL_LIB): $(PROJECT_DIR)syscalls/src/lib.rs $(PROJECT_DIR)syscalls/src/defs.rs
	cd $(PROJECT_DIR)syscalls && cargo build

clean-syscall:
	# This might clean some other OS files too.
	rm -rf $(PROJECT_DIR)target
