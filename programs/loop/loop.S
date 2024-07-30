.intel_syntax noprefix

.global _start
_start:
        # Execute the bochs break instruction so we know we made it here...
        xchg bx, bx
loop:
       # ...then immediately execute KidneyOS's custom syscall.
        mov eax, 0xf
        int 0x80
        jmp loop
