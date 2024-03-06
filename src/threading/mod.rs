
pub mod context_switch;
pub mod scheduling;
pub mod thread_control_block;
pub mod thread_functions;

use crate::threading::scheduling::initialize_scheduler;
use crate::threading::thread_control_block::*;
use crate::println;

use self::context_switch::switch_threads;
use self::scheduling::SCHEDULER;
use self::scheduling::scheduler::Scheduler;

//TEMP
fn scheduler_yield() -> () {
    unsafe {

        let scheduler = SCHEDULER.as_mut().unwrap();
        let switch_to = scheduler.pop().expect("No threads!");
        let switch_from = RUNNING_THREAD.take().expect("Why is nothing running?");
        Option::exchange();
        SCHEDULER.as_mut().unwrap().push(switch_from);
        switch_threads(switch_from, switch_to);
    }
}

pub fn test_func() {

    loop {
        println!("Hello threads!");
        scheduler_yield();
    };

}

pub fn test_halt() {

    loop {
        println!("Goodbye threads!");
        scheduler_yield();
    };

}

static mut RUNNING_THREAD: Option<ThreadControlBlock> = None;

/**
 * To be called before any other thread functions.
 * To be called with interrupts disabled.
 */
static mut THREAD_SYSTEM_INITIALIZED: bool = false;
pub fn thread_system_initialization() -> () {

    println!("Initializing Thread System...");

    // TODO: Ensure interrupts are off.

    // Initialize the TID lock.

    // Initialize the scheduler.
    initialize_scheduler();

    // Create Idle thread.

    unsafe { THREAD_SYSTEM_INITIALIZED = true; }
    println!("Finished Thread System initialization. Ready to start threading.");

}

/**
 * Enables preemptive scheduling.
 * Thread system must have been previously enabled.
 */
pub fn thread_system_start() -> () {

    if unsafe { !THREAD_SYSTEM_INITIALIZED } {
        panic!("Cannot start threading without initializing the threading system.");
    }

    // TODO: Enable interrupts.

    // TEMP.
    let tcb_1 = ThreadControlBlock::create(test_halt);
    let tcb_2 = ThreadControlBlock::create(test_func);
    unsafe {
        RUNNING_THREAD = Some(tcb_1);
        SCHEDULER.as_mut().unwrap().push(tcb_2);
    }
    scheduler_yield();

}
