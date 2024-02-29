# Variable Setup

PROJECT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))
CARGO_TARGET_DIR := build/target
PROFILE := dev

-include local.mk

export CARGO_TARGET_DIR

ifeq ($(PROFILE),dev)
ARTIFACT_DIR := $(PROJECT_DIR)$(CARGO_TARGET_DIR)/i686-unknown-kernel/debug
else ifeq ($(PROFILE),release)
ARTIFACT_DIR := $(PROJECT_DIR)$(CARGO_TARGET_DIR)/i686-unknown-kernel/release
else
$(error Unhandled profile: $(PROFILE))
endif

RAW_KERNEL := $(ARTIFACT_DIR)/kidneyos
RAW_TRAMPOLINE := $(ARTIFACT_DIR)/libkidneyos_trampoline.a
TRAMPOLINE_DIR := build/trampoline
TRAMPOLINE := $(TRAMPOLINE_DIR)/libkidneyos_trampoline.a
ISO := build/kidneyos.iso

# Rust Builds

-include $(ARTIFACT_DIR)/kidneyos.d
$(RAW_KERNEL): build-support/i686.ld Cargo.toml Cargo.lock $(TRAMPOLINE)
	cargo rustc \
	  --bin kidneyos \
	  --manifest-path Cargo.toml \
	  --profile $(PROFILE) \
	  --target build-support/i686-unknown-kernel.json \
	  -Z build-std=core,alloc \
	  -Z build-std-features=compiler-builtins-mem \
	  -- \
	  -C link-arg=-T -C link-arg=$< \
	  -C link-arg=-z -C link-arg=max-page-size=0x1000 \
	  -C link-arg=-n

-include $(ARTIFACT_DIR)/libkidneyos_trampoline.d
$(RAW_TRAMPOLINE): Cargo.toml Cargo.lock
	cargo build \
	  --manifest-path trampoline/Cargo.toml \
	  --profile $(PROFILE) \
	  --target build-support/i686-unknown-kernel.json \
	  -Z build-std=core

# Trampoline Post-Build

$(TRAMPOLINE_DIR)/libkidneyos_trampoline: $(RAW_TRAMPOLINE)
	rm -rf $@
	mkdir -p $@
	ar x $< --output=$@

$(TRAMPOLINE_DIR)/libkidneyos_trampoline_unlocalized.o: $(TRAMPOLINE_DIR)/libkidneyos_trampoline
	i686-unknown-linux-gnu-ld -r -o $@ $</*.o

$(TRAMPOLINE_DIR)/libkidneyos_trampoline.o: $(TRAMPOLINE_DIR)/libkidneyos_trampoline_unlocalized.o
	i686-unknown-linux-gnu-objcopy --keep-global-symbol _start --rename-section .text=.trampoline.text $< $@

$(TRAMPOLINE): $(TRAMPOLINE_DIR)/libkidneyos_trampoline.o
	ar crus $@ $<

# Kernel Post-Build

build/isofiles/boot/kernel.bin: $(RAW_KERNEL)
	mkdir -p build/isofiles/boot
	$(OBJCOPY) --strip-debug $< $@

build/kernel.sym: $(RAW_KERNEL)
	$(OBJCOPY) --only-keep-debug $< $@

build/isofiles/boot/grub/grub.cfg: build-support/grub.cfg
	mkdir -p build/isofiles/boot/grub
	cp $< $@

$(ISO): build/isofiles/boot/kernel.bin build/isofiles/boot/grub/grub.cfg
	grub-mkrescue -o $@ build/isofiles

# Running

.PHONY: run-bochs
run-bochs: $(ISO)
	bochs -q -f bochsrc.txt

QEMU_FLAGS := -no-reboot -no-shutdown -m 4G -d int,mmu,pcall,cpu_reset,guest_errors -cdrom $(ISO)

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

# Misc

.PHONY: test
test:
	cargo test --target i686-unknown-linux-gnu --workspace

.PHONY: clean
clean:
	cargo clean
	rm -rf build
