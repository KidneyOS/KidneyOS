#include "loader.h"

OUTPUT_FORMAT(binary)
ENTRY(_start)

SECTIONS {
	. = BOOT0_BASE;

	.boot0 : {
		boot0.o (.text)
	}

	ASSERT (. == BOOT0_BASE + BOOT0_LEN, "boot0 should be exactly BOOT0_LEN bytes.")

	.boot1 : {
		*(.text)
	}

	ASSERT (. <= BOOT0_BASE + BOOT0_LEN + BOOT1_MAX_LEN, "boot1 should be less than BOOT1_MAX_LEN bytes.")

    /DISCARD/ : {
        *(.note*)
    }
}
