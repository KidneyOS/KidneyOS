AS = i686-unknown-linux-gnu-as
LD = i686-unknown-linux-gnu-ld

.PHONY:
all: exit.elf exit.binary

exit.elf: exit.o
	$(LD) -o $@ $^

exit.binary: exit.ld exit.o
	$(LD) -o $@ -T $^

exit.o: exit.S
	$(AS) -o $@ $<

.PHONY: clean
clean:
	rm -f exit.elf exit.binary exit.o
