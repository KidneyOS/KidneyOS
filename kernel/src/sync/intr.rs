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

/// A guard for withholding interrupts.
#[derive(Default)]
pub struct InterruptsGuard(bool);

impl !Send for InterruptsGuard {}

/// Prevents interrupts from occuring until the the `InterruptsGuard` is dropped.
/// After it is dropped, the interrupts are returned to the previous state.
pub fn hold_interrupts() -> InterruptsGuard {
    let enabled = intr_get_level() == IntrLevel::IntrOn;
    let retval = InterruptsGuard(enabled);
    intr_disable();
    retval
}

impl Drop for InterruptsGuard {
    fn drop(&mut self) {
        if self.0 {
            intr_enable();
        }
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
