ENTRY(_start)
OUTPUT_FORMAT(elf32-i386)

PHYS = 0x100000;
OFFSET = 0x80000000;
PAGE_FRAME_SIZE = 4096;

SECTIONS {
    . = PHYS;

    . += SIZEOF_HEADERS;

    .multiboot2_header : {
        KEEP(*(.multiboot2_header))
    }

    ASSERT(. < PHYS + 32768, "multiboot2 header must be within the first 32768 bytes.")

    .multiboot.text ALIGN(4K) : {
        trampoline_start = .;
        *libkidneyos_trampoline.o(.text*)
    }

    .multiboot.rodata ALIGN(4K) : {
        *libkidneyos_trampoline.o(.rodata*)
    }

    .multiboot.data ALIGN(4K) : {
        trampoline_data_start = .;
        *libkidneyos_trampoline.o(.data*)
        . = ALIGN(4K);
        *libkidneyos_trampoline.o(.bss*)
        . = ALIGN(4K);
    }

    trampoline_end = .;
}

SECTIONS {
    . += OFFSET;

    .text ALIGN(4K) : AT(ADDR(.text) - OFFSET) {
        kernel_start = .;
        *(EXCLUDE_FILE(*libkidneyos_trampoline.o) .text*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - OFFSET) {
        *(EXCLUDE_FILE(*libkidneyos_trampoline.o) .rodata*)
    }

    .data ALIGN(4K) : AT(ADDR(.data) - OFFSET) {
        kernel_data_start = .;
        *(EXCLUDE_FILE(*libkidneyos_trampoline.o) .data*)
        . = ALIGN(4K);
        *(EXCLUDE_FILE(*libkidneyos_trampoline.o) .bss*)
        . = ALIGN(4K);
    }

    kernel_end = .;

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note*)
        *(.rel.eh_frame*)
    }
}
