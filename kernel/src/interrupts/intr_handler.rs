use core::arch::asm;
use kidneyos_shared::println;
use crate::drivers::ata::ata_interrupt;
use crate::drivers::input::keyboard;
use crate::interrupts::{pic, timer};
use crate::threading::scheduling;
use crate::threading::scheduling::scheduler_yield_and_die;
use crate::threading::thread_functions::landing_pad;
use crate::user_program::syscall;

/* This file contains all the interrupt handlers to be installed in the IDT when the kernel is initialized.
 * Each must be naked function with C linkage and the type fn() -> !
 */

#[naked]
pub unsafe extern "C" fn unhandled_handler() -> ! {
    fn inner() -> ! {
        panic!("unhandled interrupt");
    }

    asm!(
        "call {}",
        sym inner,
        options(noreturn),
    )
}

#[naked]
pub unsafe extern "C" fn page_fault_handler() -> ! {
    unsafe fn inner(error_code: u32, return_eip: usize) -> ! {
        if return_eip == landing_pad as usize {
            println!("Thread executed landing pad, this happens when \
                a user thread returns without terminating (ex. does not call exit(...)).");

            scheduler_yield_and_die()
        }

        let vaddr: usize;
        asm!("mov {}, cr2", out(reg) vaddr);
        panic!("page fault with error code {error_code:#b} occurred when trying to access {vaddr:#X} from instruction at {return_eip:#X}");
    }

    asm!(
        "call {}",
        sym inner,
        options(noreturn),
    )
}

#[naked]
pub unsafe extern "C" fn syscall_handler() -> ! {
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
    )
}

#[naked]
pub unsafe extern "C" fn timer_interrupt_handler() -> ! {
    asm!(
        "
        pusha
        // Push IRQ0 value onto the stack.
        push 0x0
        call {} // Update system clock
        call {} // Send EOI signal to PICs
        call {} // Yield process

        add esp, 4 // Drop arguments from stack
        popa
        iretd
        ",
        sym timer::step_sys_clock,
        sym pic::send_eoi,
        sym scheduling::scheduler_yield_and_continue,
        options(noreturn),
    )
}

#[naked]
pub unsafe extern "C" fn ide_prim_interrupt_handler() -> ! {
    asm!(
    "
    pusha
    // Push IRQ14 value onto the stack.
    push 0XE
    call {} // Send irq signal to ATA
    call {} // Send EOI signal to PICs
    call {} // Yield process

    add esp, 4 // Drop arguments from stack
    popa
    iretd
    ",
    sym ata_interrupt::on_ide_interrupt,
    sym pic::send_eoi,
    sym scheduling::scheduler_yield_and_continue,
    options(noreturn),
    )
}

#[naked]
pub unsafe extern "C" fn ide_secd_interrupt_handler() -> ! {
    asm!(
    "
    pusha
    // Push IRQ15 value onto the stack.
    push 0XF
    call {} // Send irq signal to ATA
    call {} // Send EOI signal to PICs
    call {} // Yield process

    add esp, 4 // Drop arguments from stack
    popa
    iretd
    ",
    sym ata_interrupt::on_ide_interrupt,
    sym pic::send_eoi,
    sym scheduling::scheduler_yield_and_continue,
    options(noreturn),
    )
}

#[naked]
pub unsafe extern "C" fn keyboard_handler() -> ! {
    asm!(
    "
    pusha
    // Push IRQ1 value onto the stack.
    push 0X1
    call {} // Handle keyboard interrupt
    call {} // Send EOI signal to PICs
    call {} // Yield process

    add esp, 4 // Drop arguments from stack
    popa
    iretd
    ",
    sym keyboard::atkbd::on_keyboard_interrupt,
    sym pic::send_eoi,
    sym scheduling::scheduler_yield_and_continue,
    options(noreturn),
    )
}
