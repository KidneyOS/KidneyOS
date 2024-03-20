pub mod fifo_scheduler;
pub mod scheduler;

use fifo_scheduler::FIFOScheduler;
use scheduler::Scheduler;

use alloc::boxed::Box;

use super::{context_switch::switch_threads, thread_control_block::ThreadStatus, RUNNING_THREAD};

pub static mut SCHEDULER: Option<Box<dyn Scheduler>> = None;

pub fn initialize_scheduler() {
    // SAFETY: Interrupts should be off.
    unsafe {
        SCHEDULER = Some(Box::new(FIFOScheduler::new()));
    }
}

pub fn scheduler_yield() {
    // SAFETY: Threads and Scheduler must be initialized and active.
    // As well, interupts must be off.
    unsafe {
        let scheduler = SCHEDULER.as_mut().expect("No Scheduler set up!");
        let switch_to = scheduler.pop().expect("No threads to run!");

        // The running thread is not blocked if it has called this.
        // Place it in the ready state.
        let mut running = RUNNING_THREAD.take().expect("No running thread");
        running.status = ThreadStatus::Ready;
        RUNNING_THREAD = Some(running);

        // Switch to this other thread.
        switch_threads(switch_to);
    }
}
