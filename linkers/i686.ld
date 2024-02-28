ENTRY(_start)
OUTPUT_FORMAT(elf32-i386)

SECTIONS {
    . = 0x100000;

    . += SIZEOF_HEADERS;

    .multiboot2_header : {
        KEEP(*(.multiboot2_header))
    }

    .text ALIGN(4K) : {
        __kernel_start = .;
        *(.text*)
    }

    .rodata ALIGN(4K) : {
        *(.rodata*)
    }

    .data ALIGN(4K) : {
        __kernel_data_start = .;
        *(.data*)
        . = ALIGN(4K);
        *(.bss*)
        . = ALIGN(4K);
    }

    __kernel_end = .;

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note*)
        *(.rel.eh_frame*)
    }
}
