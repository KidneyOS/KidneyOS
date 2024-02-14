ENTRY(_start)
OUTPUT_FORMAT(elf32-i386)

/* NOTE: These values must be kept synced with the values in src/mem/mod.rs */
KERNEL_OFFSET = 0x100000;
KERNEL_DATA_OFFSET = 0x180000;
KERNEL_MAX_SIZE = 0x100000;

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

    ASSERT(. <= KERNEL_DATA_OFFSET, "kernel sections before data shouldn't be bigger than 512KB.")

    . = KERNEL_DATA_OFFSET;

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

    ASSERT(. <= KERNEL_OFFSET + KERNEL_MAX_SIZE, "kernel shouldn't be bigger than 1MB.")

    /DISCARD/ : {
        *(.comment*)
        *(.eh_frame*)
        *(.gcc_except_table*)
        *(.note*)
        *(.rel.eh_frame*)
    }
}
