LD ?= i686-unknown-linux-gnu-ld
CC ?= i686-unknown-linux-gnu-cc

loader.bin: loader.ld loader.o
	$(LD) -T $^ -o $@

loader.o: loader.S loader.h
	$(CC) -c $< -o $@

.PHONY: run-qemu
run-qemu:
	qemu-system-i386 -fda loader.bin

.PHONY: run-qemu-ng
run-qemu-ng:
	# NOTE: You can quit with Ctrl-A X
	qemu-system-i386 -nographic -fda loader.bin

.PHONY: clean
clean:
	rm -f loader.bin loader.o
