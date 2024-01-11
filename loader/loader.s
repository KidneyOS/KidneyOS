.intel_syntax noprefix
.code16 # We have to start the bootloader in 16-bit mode.

.section .text
.global _start
_start:
	lea si, hello # Load address of string.
	mov ah, 0x0E # Specifies 'Write Character in TTY mode' operation.

.loop:
	lodsb # Loads value from address in si into al when in legacy mode.

	# Break if we've gotten to the '\0'.
	or al, al
	jz halt

	int 0x10 # BIOS interrupt 0x10 (Video Services).
	jmp .loop

halt:
	cli
	hlt

hello:
	.asciz "Hello world!\r\n"

	. = _start + 510
	.word 0xAA55 # Magic number to mark sector as bootable.
