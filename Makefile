-include local.mk

PROFILE ?= dev

ifeq ($(PROFILE),dev)
OUT_DIR_NAME := debug
else ifeq ($(PROFILE),release)
OUT_DIR_NAME := release
else
$(error Unhandled profile: $(PROFILE))
endif

kidneyos.iso: isofiles/boot/kernel.bin isofiles/boot/grub/grub.cfg
	grub-mkrescue -o $@ isofiles

isofiles/boot/kernel.bin: $(realpath .)/target/i686-unknown-kernel/$(OUT_DIR_NAME)/kidneyos
	$(OBJCOPY) --strip-debug $< $@

kernel.sym: $(realpath .)/target/i686-unknown-kernel/$(OUT_DIR_NAME)/kidneyos
	$(OBJCOPY) --only-keep-debug $< $@

-include target/i686-unknown-kernel/$(OUT_DIR_NAME)/kidneyos.d
$(realpath .)/target/i686-unknown-kernel/$(OUT_DIR_NAME)/kidneyos: Cargo.toml Cargo.lock
	cargo rustc \
	  --bin kidneyos \
	  --manifest-path Cargo.toml \
	  --profile $(PROFILE) \
	  --target targets/i686-unknown-kernel.json \
	  -Z build-std=core,alloc \
	  -Z build-std-features=compiler-builtins-mem \
	  -- \
	  -C link-arg=-T -C link-arg=linkers/i686.ld \
	  -C link-arg=-z -C link-arg=max-page-size=0x1000 \
	  -C link-arg=-n

.PHONY: run-bochs
run-bochs: kidneyos.iso
	bochs -q -f bochsrc.txt

QEMU_FLAGS := -no-reboot -no-shutdown -m 4G -d int,mmu,pcall,cpu_reset,guest_errors -cdrom kidneyos.iso

.PHONY: run-qemu
run-qemu: kidneyos.iso
	qemu-system-i386 $(QEMU_FLAGS)

.PHONY: run-qemu-gdb
run-qemu-gdb: kidneyos.iso kernel.sym
	qemu-system-i386 -s -S $(QEMU_FLAGS)

.PHONY: run-qemu-ng
run-qemu-ng: kidneyos.iso
	# NOTE: You can quit with Ctrl-A X
	qemu-system-i386 -nographic $(QEMU_FLAGS)

.PHONY: test
test:
	cargo test --target i686-unknown-linux-gnu --workspace

.PHONY: clean
clean:
	cargo clean
	rm -f kidneyos.iso isofiles/boot/kernel.bin kernel.sym
