all: build/basic

include ../../syscalls.mk

build:
	mkdir build

build/basic: build basic.c $(SYSCALL_LIB)
	i686-unknown-linux-gnu-gcc basic.c -o build/basic $(SYSCALL_LIB) -I ../../syscalls/include -ffreestanding -fno-stack-protector -nostdlib -e _start -nostartfiles

clean: clean-syscall
	rm -rf build
