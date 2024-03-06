
pub mod scheduler;
pub mod fifo_scheduler;

use self::scheduler::Scheduler;
use self::scheduler::NullScheduler;
use self::fifo_scheduler::FIFOScheduler;

use alloc::boxed::Box;

pub static mut SCHEDULER: Option<Box<dyn Scheduler>> = None;

pub fn initialize_scheduler() -> () {
    unsafe { SCHEDULER = Some(Box::new(FIFOScheduler::new())); }
}
