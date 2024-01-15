# This bootloader is designed to load KidneyOS. It supports the subset of
# Multiboot2 functionality required by KidneyOS. While it will leave the system
# in mostly the same state as a Multiboot2-compliant bootloader upon entry to
# the kernel, it isn't Multiboot2-compliant itself since it makes assumptions
# about what multiboot headers will be defined, and doesn't error upon
# unsupported headers. Because of this, it should only be used with KidneyOS.

.intel_syntax noprefix

#include "loader.h"
#define MAGIC_VALUE 0xAA55

.section .text
.code16

# _start is the entrypoint of the bootloader.
.global _start
_start:
	# At this point, everything from 0-0x7C00 is reserved by the hardware,
	# and boot0 is in memory at 0x7C00-0x7E00.

	# On entry, dl contains the drive number from which we were loaded. Our
	# goal is to read boot1 into memory and jump into that. We need to do
	# this instead of just doing everything in boot0 because boot0 must fit
	# within 512 bytes. boot1 is located immediately after boot0, so we want
	# to use the number in dl when reading boot1 into memory, meaning dl
	# mustn't be overwritten until we get to the point where we need to use
	# it.

	cli # Disable interrupts.

	# The BIOS will usually have started us with `jmp 0:0x7C00`, but some
	# will start us with `jmp 0x7C0:0`, which will cause issues with
	# absolute addresses. The following jump will fix this.
	ljmp 0:n
n:

	# Set up data and stack segments to both start from zero.
	xor ax, ax
	mov ds, ax
	mov ss, ax

	# Set the top of the stack (from which we'll grow downwards towards the
	# bootloader code, which should end at 0x7E00. We have to make sure the
	# stack never grows that far or else we'll corrupt stuff).
	mov esp, 0xE000

	sti # Re-enable interrupts.

	# Reset disk controller to known state.
	xor ah, ah
	int 0x13
	jc error

	# Read
	xor ax, ax
	mov es, ax # Start offset of read buffer
	mov ah, 2 # "Read Sectors" operation
	# BUG: Assumes 512 byte sector size.
	mov al, BOOT1_SECTORS # Number of sectors to read
	xor ch, ch # Cylinder number
	mov cl, 1 # Sector number
	xor dh, dh # Head number
	# dl should still contain the drive number containg boot0 and boot1 at
	# its start since it hasn't been modified since startup.
	mov bx, BOOT1_BASE # Read buffer index
	int 0x13
	jc error # TODO: Retry failures, resetting in between.

	jmp 0:BOOT1_BASE + BOOT0_LEN

error:
	# Notify BIOS that boot failed, returning control.
	int 0x18
	jmp error

# Include magic number to mark sector as bootable.
	. = _start + 510
.global magic
magic:	.word MAGIC_VALUE
