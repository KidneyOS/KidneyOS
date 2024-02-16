#define BOOT0_BASE 0x7C00
#define BOOT0_LEN 512
#define BOOT1_BASE 0xF000
#define BOOT1_MAX_LEN 0xE00
#define BOOT1_SECTORS 7
/* Kernel virtual address at which all physical memory is mapped.
   Must be aligned on a 4 MB boundary. */
#define LOADER_PHYS_BASE 0xc0000000   

#define BOCHS_BREAK xchg bx, bx
