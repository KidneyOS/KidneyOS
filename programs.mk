AS = i686-unknown-linux-gnu-as
LD = i686-unknown-linux-gnu-ld

PROGRAMS := programs/exit/exit  programs/loop/loop

programs/exit/exit: programs/exit/exit.o
	$(LD) -o $@ $^

programs/exit/exit.o: programs/exit/exit.S
	$(AS) -o $@ $<

programs/loop/loop: programs/loop/loop.o
	$(LD) -o $@ $^

programs/loop/loop.o: programs/loop/loop.S
	$(AS) -o $@ $<

.PHONY: clean
clean::
	rm -f programs/exit/exit programs/exit/exit.o
	rm -f programs/loop/loop programs/loop/loop.o
