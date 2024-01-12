.intel_syntax noprefix
.code16 # We have to start the bootloader in 16-bit mode.

.section .text
.global _start
_start:
	# Set up data and stack segments to both start from zero so we can refer
	# to absolute addresses in the following setup code.
	xorw ax, ax
	mov ds, ax
	mov ss, ax

	# Start the stack (which grows downward) at 1K.
	# NOTE: This may need to be increased if loader complexity grows.
	mov esp, 0x400

	call puts
	.asciz "Hello, world!\r\n"
	call puts
	.asciz "Halting...\r\n"

halt:
	cli
	hlt


# puts prints the null-terminated string whose data is contained in the code
# immediately following the call to puts to the screen using BIOS interrupts. If
# you want the cursor to end at the start of the next line, your string should
# end with "\r\n".
puts:
	# Use return address as the address of the string to print.
	mov si, [esp]
	mov ah, 0xE # Specify "Display Character" operation for the BIOS interrupts below.

.puts_loop:
	lodsb # Load value at address in si into al and increment si.

	# Return if this is the null-terminator.
	test al, al
	jz .puts_done

	int 0x10 # Trigger video services BIOS interrupt.
	jmp .puts_loop

.puts_done:
	# Use the updated value of si (which is now past the string) as the
	# return address.
	mov [esp], si
	ret

# Include magic number to mark sector as bootable.
	. = _start + 510
	.word 0xAA55
