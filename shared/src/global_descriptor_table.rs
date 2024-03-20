// https://wiki.osdev.org/GDT
// https://wiki.osdev.org/GDT_Tutorial

use arbitrary_int::{u2, u20};
use bitbybit::bitfield;
use core::{arch::asm, mem::size_of};

#[bitfield(u64, default = 0)]
struct SegmentDescriptor {
    #[bits([0..=15, 48..=51], rw)]
    limit: u20,
    #[bits([16..=31, 32..=39, 56..=63], rw)]
    base_low: u32,
    #[bit(40, rw)]
    accessed: bool,
    #[bit(41, rw)]
    read_write: bool,
    #[bit(42, rw)]
    direction_conforming: bool,
    #[bit(43, rw)]
    executable: bool,
    #[bit(44, rw)]
    r#type: bool,
    #[bits(45..=46, rw)]
    descriptor_privilege_level: u2,
    #[bit(47, rw)]
    present: bool,
    #[bit(53, rw)]
    long_mode: bool,
    #[bit(54, rw)]
    size: bool,
    #[bit(55, rw)]
    granularity: bool,
}

impl SegmentDescriptor {
    const UNLIMITED: Self = Self::DEFAULT
        .with_limit(u20::new(0xFFFFF))
        .with_size(true)
        .with_granularity(true);
}

#[repr(packed)]
struct GDTDescriptor {
    #[allow(unused)]
    size: u16,
    offset: u32,
}

const GDT_LEN: usize = 5;

const GDT: &[SegmentDescriptor; GDT_LEN] = &[
    // Null Descriptor
    SegmentDescriptor::DEFAULT,
    // Kernel Mode Code
    SegmentDescriptor::UNLIMITED
        .with_present(true)
        .with_type(true)
        .with_executable(true)
        // Means we can read, since this is a code segment.
        .with_read_write(true),
    // Kernel Mode Data
    SegmentDescriptor::UNLIMITED
        .with_present(true)
        .with_type(true)
        // Means we can write, since this is a data segment.
        .with_read_write(true),
    // User Mode Code
    SegmentDescriptor::UNLIMITED
        .with_present(true)
        // Allow unprivileged access.
        .with_descriptor_privilege_level(u2::new(3))
        .with_type(true)
        .with_executable(true)
        // Means we can read, since this is a code segment.
        .with_read_write(true),
    // User Mode Data
    SegmentDescriptor::UNLIMITED
        .with_present(true)
        // Allow unprivileged access.
        .with_descriptor_privilege_level(u2::new(3))
        .with_type(true)
        // Means we can write, since this is a data segment.
        .with_read_write(true),
    // TODO: may need to add task state segment here at some point.
];

static mut GDT_DESCRIPTOR: GDTDescriptor = GDTDescriptor {
    size: size_of::<[SegmentDescriptor; GDT_LEN]>() as u16 - 1,
    offset: 0, // Will fetch pointer and set at runtime below.
};

/// # Safety
///
/// Can only be executed within code that expects to have segments defined as
/// they are above in GDT.
pub unsafe fn load() {
    GDT_DESCRIPTOR.offset = GDT.as_ptr() as u32;

    // We need to use att_syntax since Rust doesn't appear to understand intel
    // long jump syntax...
    asm!(
        "
        lgdt [{}]
        ljmp $0x08, $2f
2:
        mov $0x10, %eax
        mov %eax, %ds
        mov %eax, %es
        mov %eax, %fs
        mov %eax, %gs
        mov %eax, %ss
        ",
        sym GDT_DESCRIPTOR,
        options(att_syntax),
    );
}
