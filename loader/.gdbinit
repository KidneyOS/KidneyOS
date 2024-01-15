set architecture i8086
set disassembly-flavor intel
layout asm
target remote localhost:1234
break *0x7C00
continue
