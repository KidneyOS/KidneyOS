AS = i686-unknown-linux-gnu-as
LD = i686-unknown-linux-gnu-ld

exit: exit.o
	$(LD) -o $@ $^

exit.o: exit.S
	$(AS) -o $@ $<

.PHONY: clean
clean:
	rm -f exit exit.o
