all: build/basic build/mmap

include ../../syscalls.mk

build:
	mkdir build

build/%: %.c build $(SYSCALL_LIB)
	i686-unknown-linux-gnu-gcc $< -o $@ $(SYSCALL_LIB) -I ../../syscalls/include -ffreestanding -fno-stack-protector -nostdlib -e _start -nostartfiles

clean: clean-syscall
	rm -rf build
