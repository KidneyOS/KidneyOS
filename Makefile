# Variable Setup

PROJECT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

-include local.mk

CARGO_TARGET_DIR ?= $(PROJECT_DIR)/build/target
PROFILE ?= dev

export CARGO_TARGET_DIR

ifeq ($(PROFILE),dev)
ARTIFACT_DIR := $(CARGO_TARGET_DIR)/i686-unknown-kernel/debug
else ifeq ($(PROFILE),release)
ARTIFACT_DIR := $(CARGO_TARGET_DIR)/i686-unknown-kernel/release
else
$(error Unhandled profile: $(PROFILE))
endif

RAW_KERNEL := $(ARTIFACT_DIR)/kidneyos
RAW_TRAMPOLINE := $(ARTIFACT_DIR)/libkidneyos_trampoline.a
TRAMPOLINE_DIR := build/trampoline
TRAMPOLINE := $(TRAMPOLINE_DIR)/libkidneyos_trampoline.a
ISO := build/kidneyos.iso
ATADISK := mbr_ext4_50MiB.img

.PHONY: default
default: $(ISO)

include programs.mk

# Rust Builds

-include $(ARTIFACT_DIR)/kidneyos.d
$(RAW_KERNEL): kernel/Cargo.toml Cargo.lock build-support/i686.ld $(TRAMPOLINE) $(PROGRAMS)
	cargo rustc \
	  --bin kidneyos \
	  --manifest-path $< \
	  --profile $(PROFILE) \
	  --target build-support/i686-unknown-kernel.json \
	  -Z build-std=core,alloc \
	  -Z build-std-features=compiler-builtins-mem \
	  -- \
	  -C link-arg=-T -C link-arg=build-support/i686.ld \
	  -C link-arg=-z -C link-arg=max-page-size=0x1000 \
	  -C link-arg=-n

-include $(ARTIFACT_DIR)/libkidneyos_trampoline.d
$(RAW_TRAMPOLINE): trampoline/Cargo.toml Cargo.lock
	cargo build \
	  --manifest-path $< \
	  --profile $(PROFILE) \
	  --target build-support/i686-unknown-kernel.json \
	  -Z build-std=core \
	  -Z build-std-features=compiler-builtins-mem

# Trampoline Post-Build

$(TRAMPOLINE_DIR)/libkidneyos_trampoline: $(RAW_TRAMPOLINE)
	rm -rf $@
	mkdir -p $@
	i686-unknown-linux-gnu-ar x $< --output=$@

$(TRAMPOLINE_DIR)/libkidneyos_trampoline_unlocalized.o: $(TRAMPOLINE_DIR)/libkidneyos_trampoline
	i686-unknown-linux-gnu-ld -r -o $@ $</*.o

$(TRAMPOLINE_DIR)/libkidneyos_trampoline.o: $(TRAMPOLINE_DIR)/libkidneyos_trampoline_unlocalized.o
	i686-unknown-linux-gnu-objcopy --keep-global-symbol _start --rename-section .text=.trampoline.text $< $@

$(TRAMPOLINE): $(TRAMPOLINE_DIR)/libkidneyos_trampoline.o
	ar crus $@ $<

# Kernel Post-Build

build/isofiles/boot/kernel.bin: $(RAW_KERNEL)
	mkdir -p build/isofiles/boot
	i686-unknown-linux-gnu-objcopy --strip-debug $< $@

build/kernel.sym: $(RAW_KERNEL)
	i686-unknown-linux-gnu-objcopy --only-keep-debug $< $@

build/isofiles/boot/grub/grub.cfg: build-support/grub.cfg
	mkdir -p build/isofiles/boot/grub
	cp $< $@

$(ISO): build/isofiles/boot/kernel.bin build/isofiles/boot/grub/grub.cfg
	grub-mkrescue -o $@ build/isofiles

# Disk Image
.PHONY: disk
disk:
	@echo "Generating disk image: $(ATADISK)"
	./scripts/generate-disk.bash -s 50MiB -f fat16

# Running

.PHONY: run-bochs
run-bochs: $(ISO)
	bochs -q -f bochsrc.txt

# QEMU_FLAGS := -no-reboot -no-shutdown -m 4G -d int,mmu,pcall,cpu_reset,guest_errors -cdrom $(ISO)
QEMU_FLAGS := -no-reboot -no-shutdown -m 4G -d int,mmu,pcall,cpu_reset,guest_errors -cdrom $(ISO) \
			  -drive format=raw,file=${ATADISK},if=ide \
			  -boot d \
			  -cpu Haswell,+rdrand

.PHONY: run-qemu
run-qemu: $(ISO)
	qemu-system-i386 $(QEMU_FLAGS)

.PHONY: run-qemu-gdb
run-qemu-gdb: $(ISO) build/kernel.sym
	qemu-system-i386 -s -S $(QEMU_FLAGS)

.PHONY: run-qemu-ng
run-qemu-ng: $(ISO)
	# NOTE: You can quit with Ctrl-A X
	qemu-system-i386 -nographic $(QEMU_FLAGS)

# Docs

.PHONY: docs
docs:
	mdbook build docs

.PHONY: docs-serve
docs-serve:
	mdbook serve docs

# Misc

LLVM_PROFILE_FILE := default.profraw
export LLVM_PROFILE_FILE

# NOTE: This needs to be updated if we add tests to the trampoline crate.
LLVM_PROFILE_FILES := kernel/$(LLVM_PROFILE_FILE) shared/$(LLVM_PROFILE_FILE)

.PHONY: test
test $(LLVM_PROFILE_FILES): $(PROGRAMS)
	RUSTFLAGS="-C instrument-coverage" cargo test \
	   --target i686-unknown-linux-gnu --workspace

.PHONY: report-coverage
report-coverage: $(LLVM_PROFILE_FILES)
	grcov $^ --binary-path build/target/i686-unknown-linux-gnu/debug \
	    --branch --output-path build/coverage --output-types html --source-dir .

build/coverage.md: $(LLVM_PROFILE_FILES)
	grcov $(LLVM_PROFILE_FILES) \
	    --binary-path build/target/i686-unknown-linux-gnu/debug \
	    --branch --output-path $@ --output-types markdown --source-dir .

.PHONY: print-coverage
print-coverage: build/coverage.md
	tail -n 1 $<

.PHONY: clean
clean::
	cargo clean
	rm -rf build $(LLVM_PROFILE_FILES)
