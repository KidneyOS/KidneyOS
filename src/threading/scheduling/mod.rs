pub mod fifo_scheduler;
pub mod scheduler;

use self::fifo_scheduler::FIFOScheduler;
use self::scheduler::Scheduler;

use alloc::boxed::Box;

use super::context_switch::switch_threads;
use super::thread_control_block::ThreadControlBlock;

pub static mut SCHEDULER: Option<Box<dyn Scheduler>> = None;

pub fn initialize_scheduler() {
    // SAFETY: Interrupts should be off.
    unsafe {
        SCHEDULER = Some(Box::new(FIFOScheduler::new()));
    }
}

pub fn scheduler_yield() {
    // SAFETY: Threads and Scheduler must be initialized and active.
    unsafe {
        let scheduler = SCHEDULER.as_mut().expect("No Scheduler set up!");
        let switch_to = scheduler.pop().expect("No threads to run!");

        switch_threads(switch_to);
    }
}
