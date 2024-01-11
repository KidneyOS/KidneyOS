AS ?= i686-unknown-linux-gnu-as
LD ?= i686-unknown-linux-gnu-ld

loader.bin: loader.ld loader.o
	$(LD) -T $^ -o $@

loader.o: loader.s
	$(AS) $< -o $@

.PHONY: run
run:
	qemu-system-i386 -fda loader.bin

.PHONY: clean
clean:
	rm -f loader.bin loader.o
