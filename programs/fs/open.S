.intel_syntax noprefix

.equiv O_CREAT, 0x40
.equiv SYS_OPEN, 0x5

.global _start
_start:
		mov eax, SYS_OPEN
		lea ebx, [filename]
		mov ecx, O_CREAT
		int 0x80
		cmp eax, 0
		jl exit
		
		mov eax, SYS_OPEN
		lea ebx, [filename]
		mov ecx, 0
		int 0x80
		cmp eax, 0
		jl exit
		mov eax, 0
		jmp exit
filename:
		.asciz "test.txt"
exit:
		mov ebx, eax
        mov eax, 0x1
        int 0x80
loop:
        jmp loop
