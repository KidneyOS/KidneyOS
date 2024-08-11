// https://wiki.osdev.org/Interrupt_Descriptor_Table
// https://wiki.osdev.org/Interrupts_tutorial
// https://wiki.osdev.org/Exceptions

use crate::user_program::syscall;
use arbitrary_int::{u2, u4};
use bitbybit::bitfield;
use core::{arch::asm, mem::size_of};

// use super::dev::ide::ide_interrupt_handler;

use crate::threading::scheduling;
use crate::timer;

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

// TODO: Set up stack on entry to handlers from kernel, the current behaviour is
// horribly dangerous... The current behaviour is currently safe fine for cases
// where we're entering a handler from usermode though, because when doing that
// we get the new stack from the TSS.

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

#[naked]
unsafe extern "C" fn syscall_handler() -> ! {
    asm!(
        "
        // Push arguments to stack.
        push edx
        push ecx
        push ebx
        push eax

        // TODO: We need to define what our syscall ABI is allowed to clobber
        // and what it must preserve, then actually do that. We should also
        // investigate what actual OSs do to ensure that we're not leaking
        // sensitive kernel data, even if we are respecting our ABI.

        call {}
        // eax will contain the handler's return value, which is where it should
        // remain when we return to the program.

        add esp, 16 // Drop arguments from stack.

        iretd
        ",
        sym syscall::handler,
        options(noreturn),
    );
}

#[naked]
unsafe extern "C" fn timer_interrupt_handler() -> ! {
    asm!(
        "
        // Push IRQ0 value onto the stack.
        push 0x0
        call {} // Update system clock
        call {} // Send EOI signal to PICs
        call {} // Yield process

        add esp, 4 // Drop arguments from stack
        iretd
        ",
        sym timer::step_sys_clock,
        sym timer::send_eoi,
        sym scheduling::scheduler_yield_and_continue,
        options(noreturn),
    );
}

#[naked]
pub unsafe extern "C" fn ide_interrupt_handler() -> ! {
    fn inner() -> ! {
        panic!("PIO Interrupt");
    }

    asm!(
        "call {}",
        sym inner,
        options(noreturn),
    );
}

static mut IDT_DESCRIPTOR: IDTDescriptor = IDTDescriptor {
    size: size_of::<[GateDescriptor; IDT_LEN]>() as u16 - 1,
    offset: 0, // Will fetch pointer and set at runtime below.
};

/// # Safety
///
/// Can only be executed within code that expects the interrupt handlers to be
/// defined as they are above.
pub unsafe fn load() {
    IDT_DESCRIPTOR.offset = IDT.as_ptr() as u32;

    for gate_descriptor in &mut IDT {
        *gate_descriptor = GateDescriptor::default()
            .with_offset(unhandled_handler as usize as u32)
            .with_segment_selector(0x8)
            .with_gate_type(u4::new(0xE))
            .with_descriptor_privilege_level(u2::new(3))
            .with_present(true);
    }
    IDT[0xe] = IDT[0xe].with_offset(page_fault_handler as usize as u32);
    IDT[0x20] = IDT[0x20].with_offset(timer_interrupt_handler as usize as u32); // PIC1_OFFSET (IRQ0)
    IDT[0x2E] = IDT[0x2E].with_offset(ide_interrupt_handler as usize as u32);
    IDT[0x2F] = IDT[0x2F].with_offset(ide_interrupt_handler as usize as u32);
    IDT[0x80] = IDT[0x80].with_offset(syscall_handler as usize as u32);
    asm!("lidt [{}]", sym IDT_DESCRIPTOR);
}
