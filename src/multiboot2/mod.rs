mod header;
pub mod info;

/// EXPECTED_MAGIC is the value that multiboot2 compatible bootloaders will load
/// into eax before transferring control to the OS.
pub const EXPECTED_MAGIC: usize = 0x36D76289;
