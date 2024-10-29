// https://wiki.osdev.org/GDT
// https://wiki.osdev.org/GDT_Tutorial

use crate::{
    segment::{SegmentDescriptor, SegmentSelector},
    task_state_segment::{TaskStateSegment, TASK_STATE_SEGMENT}
};
use core::{arch::asm, mem::size_of, ptr::addr_of};

#[repr(packed)]
struct GDTDescriptor {
    #[allow(unused)]
    size: u16,
    offset: u32,
}

const GDT_LEN: usize = 6;

static mut GDT: [SegmentDescriptor; GDT_LEN] = [
    // Null Descriptor
    SegmentDescriptor::default(),
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
        .with_descriptor_privilege_level(3u8)
        .with_type(true)
        .with_executable(true)
        // Means we can read, since this is a code segment.
        .with_read_write(true),
    // User Mode Data
    SegmentDescriptor::UNLIMITED
        .with_present(true)
        // Allow unprivileged access.
        .with_descriptor_privilege_level(3u8)
        .with_type(true)
        // Means we can write, since this is a data segment.
        .with_read_write(true),
    SegmentDescriptor::default()
        .with_accessed(true)
        // Executable doesn't actually mean executable here, we just have to
        // set flags in a particular way since this is a TSS.
        .with_executable(true)
        .with_limit(size_of::<TaskStateSegment>() as u32 - 1)
        .with_present(true),
];

pub const KERNEL_CODE_SELECTOR: u16 = SegmentSelector::default().with_index(1).load();
pub const KERNEL_DATA_SELECTOR: u16 = SegmentSelector::default().with_index(2).load();
pub const USER_CODE_SELECTOR: u16 = SegmentSelector::default()
    .with_requested_privilege_level(3)
    .with_index(3u16)
    .load();
pub const USER_DATA_SELECTOR: u16 = SegmentSelector::default()
    .with_requested_privilege_level(3)
    .with_index(4)
    .load();
const TSS_INDEX: usize = 5;
const TSS_SELECTOR: u16 = SegmentSelector::default()
    .with_index(TSS_INDEX as u16)
    .load();

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

    // We need to use att_syntax since Rust doesn't appear to understand Intel long jump syntax...
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
