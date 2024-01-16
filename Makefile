.PHONY: build
build:
	cargo rustc \
	  --bin kidney-os \
	  --manifest-path Cargo.toml \
	  --target targets/i686-unknown-kernel.json \
	  -Z build-std=core \
	  -- \
	  -C link-arg=-T -C link-arg=linkers/i686.ld \
	  -C link-arg=-z -C link-arg=max-page-size=0x1000 \
	  -C link-arg=-S -C link-arg=-n \
	  --emit link=isofiles/boot/kernel.bin
	grub-mkrescue -o kidneyos.iso isofiles

.PHONY: run-bochs
run-bochs:
	bochs -q -f bochsrc.txt

.PHONY: run-qemu
run-qemu:
	qemu-system-i386 -no-reboot -no-shutdown -cdrom kidneyos.iso

.PHONY: run-qemu-gdb
run-qemu-gdb:
	qemu-system-i386 -s -S -no-reboot -no-shutdown -cdrom kidneyos.iso

.PHONY: run-qemu-ng
run-qemu-ng:
	# NOTE: You can quit with Ctrl-A X
	qemu-system-i386 -nographic -no-reboot -no-shutdown -cdrom kidneyos.iso

.PHONY: test
test:
	cargo test --target i686-unknown-linux-gnu

.PHONY: clean
clean:
	cargo clean
	rm -f isofiles/boot/kernel.bin kidneyos.iso
