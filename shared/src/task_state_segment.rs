use core::mem::{size_of, transmute};

use crate::global_descriptor_table::KERNEL_DATA_SELECTOR;

#[allow(unused)]
#[repr(C, packed)]
pub struct TaskStateSegment {
    pub link: u16,
    _reserved0: u16,
    pub esp0: u32,
    pub ss0: u16,
    _reserved1: u16,
    pub esp1: u32,
    pub ss1: u16,
    _reserved2: u16,
    pub esp2: u32,
    pub ss2: u16,
    _reserved3: u16,
    pub cr3: u32,
    pub eip: u32,
    pub eflags: u32,
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub es: u16,
    _reserved4: u16,
    pub cs: u16,
    _reserved5: u16,
    pub ss: u16,
    _reserved6: u16,
    pub ds: u16,
    _reserved7: u16,
    pub fs: u16,
    _reserved8: u16,
    pub gs: u16,
    _reserved9: u16,
    pub ldtr: u16,
    _reserved11: u32,
    pub iopb: u16,
    pub ssp: u32,
}

pub static mut TASK_STATE_SEGMENT: TaskStateSegment = {
    // Initialize zeroed TSS and set only the relevant fields.
    let mut tss: TaskStateSegment = unsafe { transmute([0_u8; size_of::<TaskStateSegment>()]) };
    tss.ss0 = KERNEL_DATA_SELECTOR;
    tss.iopb = size_of::<TaskStateSegment>() as u16;
    tss
};
