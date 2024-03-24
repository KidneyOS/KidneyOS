set architecture i8086
set disassembly-flavor intel
layout split
file build/isofiles/boot/kernel.bin
symbol-file build/kernel.sym
target remote localhost:1234
break _start
continue
