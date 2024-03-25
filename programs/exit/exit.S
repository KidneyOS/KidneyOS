.intel_syntax noprefix

.global _start
_start:
        # Execute the bochs break instruction so we know we made it here...
        xchg bx, bx
        # ...then immediately execute KidneyOS's exit syscall.
        mov eax, 1
        syscall
