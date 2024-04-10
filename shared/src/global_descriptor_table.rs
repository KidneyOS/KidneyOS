// https://wiki.osdev.org/GDT
// https://wiki.osdev.org/GDT_Tutorial

use arbitrary_int::{u13, u2, u20};
use bitbybit::bitfield;
use core::{arch::asm, mem::size_of};

use crate::segment::SegmentSelector;

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

static mut GDT: [SegmentDescriptor; GDT_LEN] = [
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

pub const KERNEL_CODE_SELECTOR: u16 = SegmentSelector::DEFAULT.with_index(u13::new(1)).raw_value();
pub const KERNEL_DATA_SELECTOR: u16 = SegmentSelector::DEFAULT.with_index(u13::new(2)).raw_value();
pub const USER_CODE_SELECTOR: u16 = SegmentSelector::DEFAULT
    .with_requested_privilege_level(u2::new(3))
    .with_index(u13::new(3))
    .raw_value();
pub const USER_DATA_SELECTOR: u16 = SegmentSelector::DEFAULT
    .with_requested_privilege_level(u2::new(3))
    .with_index(u13::new(4))
    .raw_value();

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
        lgdt 0({0})
        ljmp ${code_selector}, $2f
2:
        mov ${data_selector}, {0}
        mov {0}, %ds
        mov {0}, %es
        mov {0}, %fs
        mov {0}, %gs
        mov {0}, %ss
        ",
        in(reg) &GDT_DESCRIPTOR as *const GDTDescriptor as usize,
        code_selector = const KERNEL_CODE_SELECTOR,
        data_selector = const KERNEL_DATA_SELECTOR,
        options(att_syntax),
    );
}
