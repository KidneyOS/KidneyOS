.intel_syntax noprefix

.global _start
_start:
        # Execute the bochs break instruction so we know we made it here
        xchg bx, bx

        # Fork the process by executing KidneyOS's syscall
        mov eax, 0x2
        int 0x80

        cmp eax, 0x0
        je child

        # Wait for the parent by moving return value (parent PID)
        # into first argument reg

        mov ebx, eax
        mov eax, 0x7
        int 0x80

        # exit with the parent's exit code
        mov ebx, eax
        mov eax, 0x1
        int 0x80

        # Hang if we make it back.
loop:
        jmp loop


child:
        # Exit the process with code 1 if it is the parent
        mov eax, 0x1
        mov ebx, 0x1
        int 0x80

        jmp loop