pub mod context_switch;
pub mod scheduling;
pub mod thread_control_block;
pub mod thread_functions;

use crate::println;
use alloc::boxed::Box;

use crate::threading::scheduling::initialize_scheduler;
use crate::threading::thread_control_block::{ThreadControlBlock, Tid};

use self::scheduling::{scheduler_yield, SCHEDULER};

pub fn test_func() {
    loop {
        println!("Hello threads!");
        scheduler_yield();
    }
}

pub fn test_func_2() {
    loop {
        println!("Goodbye threads!");
        scheduler_yield();
    }
}

static mut RUNNING_THREAD: Option<Box<ThreadControlBlock>> = None;

/**
 * To be called before any other thread functions.
 * To be called with interrupts disabled.
 */
static mut THREAD_SYSTEM_INITIALIZED: bool = false;
pub fn thread_system_initialization() {
    println!("Initializing Thread System...");

    // TODO: Ensure interrupts are off.

    // Initialize the TID lock.

    // Initialize the scheduler.
    initialize_scheduler();

    // Create Idle thread.

    // SAFETY: Interrupts must be disabled.
    unsafe {
        THREAD_SYSTEM_INITIALIZED = true;
    }
    println!("Finished Thread System initialization. Ready to start threading.");
}

/**
 * Enables preemptive scheduling.
 * Thread system must have been previously enabled.
 */
pub fn thread_system_start() -> ! {
    // SAFETY: Interrupts must be disabled.
    if unsafe { !THREAD_SYSTEM_INITIALIZED } {
        panic!("Cannot start threading without initializing the threading system.");
    }

    // TODO: Enable interrupts.

    // We must 'turn the kernel thread into a thread'.
    // This amounts to just making a TCB that will be in control of the kernel stack and will
    // never exit.
    // This thread also does not need to enter the `run_thread` function.
    // SAFETY: The kernel thread's stack will be set up by the context switch following.
    let tcb_kernel = unsafe { ThreadControlBlock::create_kernel_thread() };

    // TEMP.
    let tcb_1 = ThreadControlBlock::create(test_func);
    let tcb_2 = ThreadControlBlock::create(test_func_2);

    // SAFETY: Interrupts must be disabled.
    unsafe {
        RUNNING_THREAD = Some(Box::new(tcb_kernel));
        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(Box::new(tcb_1));
        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(Box::new(tcb_2));
    }

    // Start threading by running the root thread.
    scheduler_yield();

    // Eventually, the scheduler may run the kernel thread again.
    // We may later replace this with code to clean up the kernel resources (`thread_exit` would not work).
    // For now we will just yield continually.
    loop {
        println!("It's me! The kernel thread!\n");
        scheduler_yield();
    }

    // This function never returns.
}
