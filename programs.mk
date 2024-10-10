PROGRAMS := exit example_c example_rust execve

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

execve:
	# We don't want to export CARGO_TARGET_DIR to our destination make.
	unset CARGO_TARGET_DIR && cd programs/execve && make

.PHONY: clean
clean::
	cd programs/exit && make clean
	cd programs/example_c && make clean
	# We don't want to export CARGO_TARGET_DIR to our destination make.
	unset CARGO_TARGET_DIR && cd programs/example_rust && make clean
	unset CARGO_TARGET_DIR && cd programs/execve && make clean
