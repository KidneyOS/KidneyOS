all: build/example_c

include ../../syscalls.mk

build:
	mkdir build

build/example_c: build main.c $(SYSCALL_LIB)
	i686-unknown-linux-gnu-gcc main.c -o build/example_c $(SYSCALL_LIB) -fno-stack-protector -I ../../syscalls/include -ffreestanding -nostdlib -e _start -nostartfiles

clean: clean-syscall
	rm -rf build
