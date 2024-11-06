mod fifo_scheduler;
mod scheduler;

pub use fifo_scheduler::FIFOScheduler;
pub use scheduler::Scheduler;

use alloc::boxed::Box;

use super::{context_switch::switch_threads, thread_control_block::ThreadStatus};
use crate::interrupts::{intr_get_level, mutex_irq::hold_interrupts, IntrLevel};
use crate::system::unwrap_system_mut;

pub fn create_scheduler() -> Box<dyn Scheduler> {
    assert_eq!(intr_get_level(), IntrLevel::IntrOff);

    // SAFETY: Interrupts should be off.
    Box::new(FIFOScheduler::new())
}

/// Voluntarily relinquishes control of the CPU to another processor in the scheduler.
fn scheduler_yield(status_for_current_thread: ThreadStatus) {
    let _guard = hold_interrupts();

    // SAFETY: Threads and Scheduler must be initialized and active.
    // Interrupts must be disabled.
    unsafe {
        let scheduler = unwrap_system_mut().threads.scheduler.as_mut();

        while let Some(switch_to) = scheduler.pop() {
            let is_blocked = {
                let status = &switch_to.as_ref().read().status;
                *status == ThreadStatus::Blocked
            };

            if is_blocked {
                scheduler.push(switch_to);
                continue;
            }

            switch_threads(status_for_current_thread, switch_to);
            break;
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
