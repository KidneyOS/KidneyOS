AS = i686-unknown-linux-gnu-as
LD = i686-unknown-linux-gnu-ld

PROGRAMS := programs/syscall/syscall

programs/syscall/syscall: programs/syscall/syscall.o
	$(LD) -o $@ $^

programs/syscall/syscall.o: programs/syscall/syscall.S
	$(AS) -o $@ $<

.PHONY: clean
clean::
	rm -f programs/syscall/syscall programs/syscall/syscall.o
