.intel_syntax noprefix

#include "loader.h"

.section .text
.code16

_start:
	# At this point, everything from 0-0x7C00 is reserved by the hardware,
	# boot0 is in memory at 0x7C00-0x7E00, and boot1 is in memory at
	# 0xF000-0x10000. boot0 is safe to overwrite though, because we'll never
	# return there. Our stack starts at 0xE000 and grows downwards.

	BOCHS_BREAK
	BOCHS_BREAK
	BOCHS_BREAK
