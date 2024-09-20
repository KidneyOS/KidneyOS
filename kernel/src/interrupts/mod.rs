pub mod idt;
pub mod mutex_irq;
pub mod pic;

mod intr_handler;
mod timer;

use core::{
    arch::asm,
    sync::atomic::{compiler_fence, Ordering},
};

#[allow(unused)]
#[derive(Debug, PartialEq)]
pub enum IntrLevel {
    IntrOn,
    IntrOff,
}

#[allow(unused)]
pub fn intr_get_level() -> IntrLevel {
    let flags: u32;
    unsafe {
        asm!(
        "pushfd",
        "mov {}, [esp]",
        "popfd",
        out(reg) flags
        );
    }

    if flags & (1 << 9) != 0 {
        IntrLevel::IntrOn
    } else {
        IntrLevel::IntrOff
    }
}

#[inline(always)]
pub fn intr_enable() {
    compiler_fence(Ordering::SeqCst);
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

#[inline(always)]
pub fn intr_disable() {
    unsafe {
        asm!("cli", options(nomem, nostack));
    }
    compiler_fence(Ordering::SeqCst);
}
