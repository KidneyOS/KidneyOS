# System calls

System calls are performed using the `int 0x80` instruction. The system call number should be placed in the `eax` register, and the arguments should be placed in `ebx`, `ecx`, and `edx`.
Where possible, system call numbers are chosen to be the same as on Linux.
For system calls with more than three arguments, a pointer to a structure containing all of the arguments is used instead.
The return value of the system call is placed in `eax`. A negative return value indicates an error code, for example, `eax = -EIO` indicates an I/O error.

A small wrapper library for system calls is included in the `syscalls` directory of KidneyOS, which defines a function for each system call and defines
the relevant constants for making system calls (e.g. `CLOCK_MONOTONIC`).
