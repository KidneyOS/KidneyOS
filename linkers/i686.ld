ENTRY(_start)
OUTPUT_FORMAT(elf32-i386)

KERNEL_OFFSET = 0x100000;

SECTIONS {
    . = KERNEL_OFFSET;

    . += SIZEOF_HEADERS;

    .multiboot2_header : {
        KEEP(*(.multiboot2_header))
    }

    .text ALIGN(4K) : {
        __text_start = .;
        *(.text*)
        __usercopy_start = .;
        *(.usercopy-fns)
        __usercopy_end = .;
    }

    .rodata ALIGN(4K) : {
        __text_end = .;
        __rodata_start = .;
        *(.rodata*)
    }

    .data ALIGN(4K) : {
        __rodata_end = .;
        __data_start = .;
        *(.data*)
        . = ALIGN(4K);
        __data_end = .;
        __bss_start = .;
        *(.bss*)
        . = ALIGN(4K);
    }

    __end = .;

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note*)
        *(.rel.eh_frame*)
    }
}
