.intel_syntax noprefix

.global _start
_start:
        # Execute the bochs break instruction so we know we made it here
        xchg bx, bx

        # Fork the process by executing KidneyOS's syscall
        mov eax, 0x2
        int 0x80

        # Pass an exit code of the value returned by the previous syscall
        mov ebx, eax
        mov eax, 0x1
        int 0x80

        # Hang if we make it back.
loop:
        jmp loop
