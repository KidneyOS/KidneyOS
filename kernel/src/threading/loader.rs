#![allow(dead_code)] // Suppress unused warnings

// Constants fixed by the PC BIOS.
/// Physical address of loader's base.
pub const LOADER_BASE: u32 = 0x7c00;
/// Physical address of end of loader.
pub const LOADER_END: u32 = 0x7e00;

// Physical address of kernel base.
/// 128 kB.
pub const LOADER_KERN_BASE: u32 = 0x20000;

/// Kernel virtual address at which all physical memory is mapped.
/// Must be aligned on a 4 MB boundary.
///
/// 3 GB.
pub const LOADER_PHYS_BASE: u32 = 0xc0000000;

// Important loader physical addresses.
/// 0xaa55 BIOS signature.
pub const LOADER_SIG: u32 = LOADER_END - LOADER_SIG_LEN;
/// Partition table.
pub const LOADER_PARTS: u32 = LOADER_SIG - LOADER_PARTS_LEN;
/// Command-line args.
pub const LOADER_ARGS: u32 = LOADER_PARTS - LOADER_ARGS_LEN;
/// Number of args.
pub const LOADER_ARG_CNT: u32 = LOADER_ARGS - LOADER_ARG_CNT_LEN;

// Sizes of loader data structures.
pub const LOADER_SIG_LEN: u32 = 2;
pub const LOADER_PARTS_LEN: u32 = 64;
pub const LOADER_ARGS_LEN: u32 = 128;
pub const LOADER_ARG_CNT_LEN: u32 = 4;

// GDT selectors defined by loader.
// More selectors are defined by userprog/gdt.h.
/// Null selector.
pub const SEL_NULL: u16 = 0x00;
/// Kernel code selector.
pub const SEL_KCSEG: u16 = 0x08;
/// Kernel data selector.
pub const SEL_KDSEG: u16 = 0x10;

extern "C" {
    // Amount of physical memory, in 4 kB pages.
    pub static mut init_ram_pages: u32;
}
