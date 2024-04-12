.intel_syntax noprefix

.global _start
_start:
        # Execute the bochs break instruction so we know we made it here...
        xchg bx, bx
        # ...then immediately execute KidneyOS's exit syscall, passing an exit
        # code of 0 to indicate success.
        mov eax, 1
        mov ebx, 0
        int 0x80

        # Hang if we make it back, though we shouldn't.
loop:
        jmp loop
