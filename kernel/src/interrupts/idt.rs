// https://wiki.osdev.org/Interrupt_Descriptor_Table
// https://wiki.osdev.org/Interrupts_tutorial
// https://wiki.osdev.org/Exceptions

use core::{arch::asm, mem::size_of};
use kidneyos_shared::{bit_array::BitArray, bitfield};
use paste::paste;

use crate::interrupts::intr_handler::{
    ide_prim_interrupt_handler, ide_secd_interrupt_handler, keyboard_handler, page_fault_handler,
    syscall_handler, timer_interrupt_handler, unhandled_handler,
};

bitfield!(
    GateDescriptor, u64
    {
        (u16, offset_low, 0, 15),
        (u16, offset_high, 48, 63),
        (u16, segment_selector, 16, 31),
        (u8, gate_type, 40, 43),
        (u8, descriptor_privilege_level, 45, 46),
    }
    { (present, 47) }
);

impl GateDescriptor {
    #[allow(dead_code)]
    pub fn offset(&self) -> u32 {
        ((self.offset_high() as u32) << 16) | (self.offset_low() as u32)
    }

    #[allow(dead_code)]
    pub fn with_offset(self, value: u32) -> Self {
        self.with_offset_low(value as u16)
            .with_offset_high((value >> 16) as u16)
    }
}

#[repr(packed)]
struct IDTDescriptor {
    #[allow(unused)]
    size: u16,
    offset: u32,
}

const IDT_LEN: usize = 256;
static mut IDT: [GateDescriptor; IDT_LEN] = [GateDescriptor::default(); IDT_LEN];

// TODO: Set up stack on entry to handlers from kernel, the current behaviour is
// horribly dangerous... The current behaviour is currently safe fine for cases
// where we're entering a handler from usermode though, because when doing that
// we get the new stack from the TSS.

static mut IDT_DESCRIPTOR: IDTDescriptor = IDTDescriptor {
    size: size_of::<[GateDescriptor; IDT_LEN]>() as u16 - 1,
    offset: 0, // Will fetch pointer and set at runtime below.
};

/// # Safety
///
/// Can only be executed within code that expects the interrupt handlers to be
/// defined as they are described in intr_handler.rs
pub unsafe fn load() {
    IDT_DESCRIPTOR.offset = IDT.as_ptr() as u32;

    for gate_descriptor in &mut IDT {
        *gate_descriptor = GateDescriptor::default()
            .with_offset(unhandled_handler as usize as u32)
            .with_segment_selector(0x8)
            .with_gate_type(0xEu8)
            .with_descriptor_privilege_level(3u8)
            .with_present(true);
    }
    IDT[0xe] = IDT[0xe].with_offset(page_fault_handler as usize as u32);
    IDT[0x20] = IDT[0x20].with_offset(timer_interrupt_handler as usize as u32); // PIC1_OFFSET (IRQ0)
    IDT[0x21] = IDT[0x21].with_offset(keyboard_handler as usize as u32);
    IDT[0x2E] = IDT[0x2E].with_offset(ide_prim_interrupt_handler as usize as u32); // IDE Primary (IRQ14)
    IDT[0x2F] = IDT[0x2F].with_offset(ide_secd_interrupt_handler as usize as u32); // IDE Secondary (IRQ15)
    IDT[0x80] = IDT[0x80].with_offset(syscall_handler as usize as u32);

    asm!("lidt [{}]", sym IDT_DESCRIPTOR);
}
