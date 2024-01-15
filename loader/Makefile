LD ?= i686-unknown-linux-gnu-ld
CC ?= i686-unknown-linux-gnu-cc

CFLAGS = -Wall -Werror -Wa,--fatal-warnings
LDFLAGS = --fatal-warnings

loader.bin: loader.lds.s loader.h boot0.o boot1.o
	$(LD) -T $^ -o $@

loader.lds.s: loader.h

boot0.o boot1.o: loader.h

%.o: %.S
	$(CC) $(CFLAGS) -c $< -o $@

.PHONY: run-bochs
run-bochs:
	bochs -q -f bochsrc.txt

.PHONY: run-qemu
run-qemu:
	qemu-system-i386 -no-reboot -no-shutdown -fda loader.bin

.PHONY: run-qemu-gdb
run-qemu-gdb:
	qemu-system-i386 -s -S -no-reboot -no-shutdown -fda loader.bin

.PHONY: run-qemu-ng
run-qemu-ng:
	# NOTE: You can quit with Ctrl-A X
	qemu-system-i386 -nographic -no-reboot -no-shutdown -fda loader.bin

.PHONY: clean
clean:
	rm -f loader.bin loader.lds.s *.o
