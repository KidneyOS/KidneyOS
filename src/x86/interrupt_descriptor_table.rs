// https://wiki.osdev.org/Interrupt_Descriptor_Table
// https://wiki.osdev.org/Interrupts_tutorial

use arbitrary_int::{u2, u4};
use bitbybit::bitfield;
use core::{arch::asm, mem::size_of};

#[repr(align(8))]
#[bitfield(u64, default = 0)]
struct GateDescriptor {
    #[bits([0..=15, 48..=63], rw)]
    offset: u32,
    #[bits(16..=31, rw)]
    segment_selector: u16,
    #[bits(40..=43, rw)]
    gate_type: u4,
    #[bits(45..=46, rw)]
    descriptor_privilege_level: u2,
    #[bit(47, rw)]
    present: bool,
}

#[repr(packed)]
struct IDTDescriptor {
    #[allow(unused)]
    size: u16,
    offset: u32,
}

const IDT_LEN: usize = 256;
static mut IDT: [GateDescriptor; IDT_LEN] = [GateDescriptor::DEFAULT; IDT_LEN];

// TODO: These handlers' usage of the stack might not be safe...

#[naked]
unsafe extern "C" fn unhandled_handler() -> ! {
    fn inner() -> ! {
        panic!("unhandled interrupt");
    }

    asm!(
        "call {}",
        sym inner,
        options(noreturn),
    );
}

#[naked]
unsafe extern "C" fn page_fault_handler() -> ! {
    unsafe fn inner(error_code: u32, return_eip: usize) -> ! {
        let vaddr: usize;
        asm!("mov {}, cr2", out(reg) vaddr);
        panic!("page fault with error code {error_code:#b} occurred when trying to access {vaddr:#X} from instruction at {return_eip:#X}");
    }

    asm!(
        "call {}",
        sym inner,
        options(noreturn),
    );
}

#[allow(clippy::cast_possible_truncation)]
static mut IDT_DESCRIPTOR: IDTDescriptor = IDTDescriptor {
    size: size_of::<[GateDescriptor; IDT_LEN]>() as u16 - 1,
    offset: 0, // Will fetch pointer and set at runtime below.
};

pub unsafe fn load() {
    // u32 is wide enough since we're writing 32-bit code.
    #![allow(clippy::fn_to_numeric_cast_with_truncation)]

    IDT_DESCRIPTOR.offset = IDT.as_ptr() as u32;

    for gate_descriptor in &mut IDT {
        *gate_descriptor = GateDescriptor::default()
            .with_offset(unhandled_handler as u32)
            .with_segment_selector(0x8)
            .with_gate_type(u4::new(0xE))
            .with_descriptor_privilege_level(u2::new(3))
            .with_present(true);
    }
    IDT[0x13] = IDT[0x13].with_offset(page_fault_handler as u32);

    asm!("lidt [{}]", sym IDT_DESCRIPTOR);
}
