mod fifo_scheduler;
mod scheduler;

pub use fifo_scheduler::FIFOScheduler;
pub use scheduler::Scheduler;

use alloc::boxed::Box;

use crate::sync::intr::{hold_interrupts, intr_get_level, IntrLevel};

use super::{context_switch::switch_threads, thread_control_block::ThreadStatus};

pub static mut SCHEDULER: Option<Box<dyn Scheduler>> = None;

pub fn initialize_scheduler() {
    assert_eq!(intr_get_level(), IntrLevel::IntrOff);

    // SAFETY: Interrupts should be off.
    unsafe {
        SCHEDULER = Some(Box::new(FIFOScheduler::new()));
    }
}

/// Voluntarily relinquishes control of the CPU to another processor in the scheduler.
fn scheduler_yield(status_for_current_thread: ThreadStatus) {
    let _guard = hold_interrupts();

    // SAFETY: Threads and Scheduler must be initialized and active.
    // Interrupts must be disabled.
    unsafe {
        let scheduler = SCHEDULER.as_mut().expect("No Scheduler set up!");
        let switch_to_option = scheduler.pop();

        // Do not switch to ourselves.
        if let Some(switch_to) = switch_to_option {
            // Switch to this other thread.
            switch_threads(status_for_current_thread, switch_to);
        }
    }

    // Note: _guard falls out of scope and re-enables interrupts if previously enabled
}

// Voluntarily relinquishes control of the CPU and marks current thread as ready.
pub fn scheduler_yield_and_continue() {
    scheduler_yield(ThreadStatus::Ready);
}

/// Voluntarily relinquishes control of the CPU and marks the current thread to die.
pub fn scheduler_yield_and_die() -> ! {
    scheduler_yield(ThreadStatus::Dying);

    panic!("A thread was rescheduled after dying.");
}

/// Voluntarily relinquishes control of the CPU and marks the current thread as blocked.
#[allow(unused)]
pub fn scheduler_yield_and_block() {
    scheduler_yield(ThreadStatus::Blocked);
}
