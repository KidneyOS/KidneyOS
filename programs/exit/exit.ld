ENTRY(_start)
OUTPUT_FORMAT(binary)

SECTIONS {
    .text : { *(.text*) }

    /DISCARD/ : { *(.note*) }
}
