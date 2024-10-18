// https://wiki.osdev.org/GDT
// https://wiki.osdev.org/GDT_Tutorial

use crate::{
    segment::SegmentSelector,
    task_state_segment::{TaskStateSegment, TASK_STATE_SEGMENT},
};
use arbitrary_int::{u2, u4, u13, u20};
use bitfield::bitfield;
use core::{arch::asm, mem::size_of, ptr::addr_of};

bitfield! {
    struct SegmentDescriptor(u64);
    impl Debug;
    u16, limit_low, set_limit_low: 15, 0;
    u4, limit_high, set_limit_high: 51, 48;
    u16, base_low, set_base_low: 31, 16;
    u8, base_mid, set_base_mid: 39, 32;
    u8, base_high, set_base_high: 63, 56;
    accessed, set_accessed: 40;
    read_write, set_read_write: 41;
    direction_conforming, set_direction_conforming: 42;
    executable, set_executable: 43;
    r#type, set_type: 44;
    u2, descriptor_privilege_level, set_descriptor_privilege_level: 46, 45;
    present, set_present: 47;
    long_mode, set_long_mode: 53;
    size, set_size: 54;
    granularity, set_granularity: 55;
}

impl SegmentDescriptor {
    // Combined getter for `limit`
    fn limit(&self) -> u20 {
        (u20::widen(self.limit()) << 16) | (u20::extract_u16(self.limit_low(), 0))
    }

    // Combined setter for `limit`
    fn set_limit(&mut self, value: u20) {
        self.set_limit_low(value.into());
        self.set_limit_high(value.into());
    }

    // Combined getter for `base`
    fn base(&self) -> u32 {
        ((self.base_high() as u32) << 24) | ((self.base_mid() as u32) << 16) | (self.base_low() as u32)
    }

    // Combined setter for `base`
    fn set_base(&mut self, value: u32) {
        self.set_base_low(value as u16);
        self.set_base_mid((value >> 16) as u8);
        self.set_base_high((value >> 24) as u8);
    }
}

impl SegmentDescriptor {
    const UNLIMITED: Self = Self(0)
        .set_limit(u20::new(0xFFFFF))
        .set_size(true)
        .set_granularity(true);
}

#[repr(packed)]
struct GDTDescriptor {
    #[allow(unused)]
    size: u16,
    offset: u32,
}

const GDT_LEN: usize = 6;

static mut GDT: [SegmentDescriptor; GDT_LEN] = [
    // Null Descriptor
    SegmentDescriptor(0),
    // Kernel Mode Code
    SegmentDescriptor::UNLIMITED
        .set_present(true)
        .set_type(true)
        .set_executable(true)
        // Means we can read, since this is a code segment.
        .set_read_write(true),
    // Kernel Mode Data
    SegmentDescriptor::UNLIMITED
        .set_present(true)
        .set_type(true)
        // Means we can write, since this is a data segment.
        .set_read_write(true),
    // User Mode Code
    SegmentDescriptor::UNLIMITED
        .set_present(true)
        // Allow unprivileged access.
        .set_descriptor_privilege_level(u2::new(3))
        .set_type(true)
        .set_executable(true)
        // Means we can read, since this is a code segment.
        .set_read_write(true),
    // User Mode Data
    SegmentDescriptor::UNLIMITED
        .set_present(true)
        // Allow unprivileged access.
        .set_descriptor_privilege_level(u2::new(3))
        .set_type(true)
        // Means we can write, since this is a data segment.
        .set_read_write(true),
    SegmentDescriptor(0)
        .set_accessed(true)
        // Executable doesn't actually mean executable here, we just have to
        // set flags in a particular way since this is a TSS.
        .set_executable(true)
        .set_limit(u20::new(size_of::<TaskStateSegment>() as u32 - 1))
        .set_present(true),
];

pub const KERNEL_CODE_SELECTOR: u16 = SegmentSelector(0).set_index(u13::new(1)).raw_value();
pub const KERNEL_DATA_SELECTOR: u16 = SegmentSelector(0).set_index(u13::new(2)).raw_value();
pub const USER_CODE_SELECTOR: u16 = SegmentSelector(0)
    .set_requested_privilege_level(u2::new(3))
    .set_index(u13::new(3))
    .raw_value();
pub const USER_DATA_SELECTOR: u16 = SegmentSelector(0)
    .set_requested_privilege_level(u2::new(3))
    .set_index(u13::new(4))
    .raw_value();
const TSS_INDEX: usize = 5;
const TSS_SELECTOR: u16 = SegmentSelector(0)
    .set_index(u13::new(TSS_INDEX as u16))
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
    GDT[TSS_INDEX] = GDT[TSS_INDEX].with_base(addr_of!(TASK_STATE_SEGMENT).cast::<u8>() as u32);
    GDT_DESCRIPTOR.offset = GDT.as_ptr() as u32;

    // We need to use att_syntax since Rust doesn't appear to understand intel
    // long jump syntax...
    asm!(
        "
        lgdt 0({0})
        ltr {1:x}
        ljmp ${code_selector}, $2f
2:
        mov ${data_selector}, {0}
        mov {0}, %ds
        mov {0}, %es
        mov {0}, %fs
        mov {0}, %gs
        mov {0}, %ss
        ",
        in(reg) &GDT_DESCRIPTOR as *const _ as usize,
        in(reg) TSS_SELECTOR,
        code_selector = const KERNEL_CODE_SELECTOR,
        data_selector = const KERNEL_DATA_SELECTOR,
        options(att_syntax),
    );
}
