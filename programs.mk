PROGRAMS := exit example_c example_rust

programs/syscall/syscall: programs/syscall/syscall.o
	$(LD) -o $@ $^

programs/syscall/syscall.o: programs/syscall/syscall.S
	$(AS) -o $@ $<

.PHONY: clean
clean::
	rm -f programs/syscall/syscall programs/syscall/syscall.o

.PHONY: programs
programs: $(PROGRAMS)

.PHONY: $(PROGRAMS)

exit:
	cd programs/exit && make

example_c:
	cd programs/example_c && make

example_rust:
	# We don't want to export CARGO_TARGET_DIR to our destination make.
	unset CARGO_TARGET_DIR && cd programs/example_rust && make

.PHONY: clean
clean::
	cd programs/exit && make clean
	cd programs/example_c && make clean
	# We don't want to export CARGO_TARGET_DIR to our destination make.
	unset CARGO_TARGET_DIR && cd programs/example_rust && make clean
