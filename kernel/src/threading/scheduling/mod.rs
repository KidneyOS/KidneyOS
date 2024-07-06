mod fifo_scheduler;
mod scheduler;

pub use fifo_scheduler::FIFOScheduler;
pub use scheduler::Scheduler;

use alloc::boxed::Box;

use crate::sync::intr::{intr_disable, intr_enable, intr_get_level, IntrLevel};

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
pub fn scheduler_yield() {
    intr_disable();

    // SAFETY: Threads and Scheduler must be initialized and active.
    // Interrupts must be disabled.
    unsafe {
        let scheduler = SCHEDULER.as_mut().expect("No Scheduler set up!");
        let switch_to = scheduler.pop().expect("No threads to run!");

        // Switch to this other thread.
        // Since this is a voluntary switch, the current thread will be ready to run again.
        switch_threads(ThreadStatus::Ready, switch_to);
    }

    intr_enable();
}
