use core::sync::atomic::AtomicU8;

use crate::timer::set_enabled;

static PREMPTION_COUNT: AtomicU8 = AtomicU8::new(0);

pub fn hold_preemption() -> PreemptionGuard {
    let prev = PREMPTION_COUNT.fetch_add(1, core::sync::atomic::Ordering::SeqCst);

    let guard = PreemptionGuard {
        preemption_was_enabled: prev == 0,
    };

    if guard.preemption_was_enabled {
        unsafe {
            set_enabled(false);
        }
    } else if prev == u8::MAX {
        panic!("BUG: overflow");
    }
    guard
}

/// A guard type that ensures preemption is disabled as long as it is held.
pub struct PreemptionGuard {
    /// Preemption enabled when this guard was created
    preemption_was_enabled: bool,
}

impl !Send for PreemptionGuard {}

impl PreemptionGuard {
    pub fn preemption_was_enabled(&self) -> bool {
        self.preemption_was_enabled
    }
}

impl Drop for PreemptionGuard {
    fn drop(&mut self) {
        let prev = PREMPTION_COUNT.fetch_sub(1, core::sync::atomic::Ordering::SeqCst);

        if prev == 1 {
            unsafe {
                set_enabled(true);
            }
        } else if prev == 0 {
            panic!("BUG: underflow");
        }
    }
}

#[allow(unused)]
pub fn preemption_enabled() -> bool {
    PREMPTION_COUNT.load(core::sync::atomic::Ordering::SeqCst) == 0
}
