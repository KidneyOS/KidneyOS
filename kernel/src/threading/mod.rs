mod context_switch;
pub mod scheduling;
mod thread_control_block;
mod thread_functions;

use crate::{
    paging::PageManager,
    sync::intr::{intr_enable, intr_get_level, IntrLevel},
};
use alloc::boxed::Box;
use kidneyos_shared::println;
use scheduling::{initialize_scheduler, scheduler_yield, SCHEDULER};
use thread_control_block::{ThreadControlBlock, Tid};

static mut RUNNING_THREAD: Option<Box<ThreadControlBlock>> = None;

/// To be called before any other thread functions.
/// To be called with interrupts disabled.
static mut THREAD_SYSTEM_INITIALIZED: bool = false;
pub fn thread_system_initialization() {
    println!("Initializing Thread System...");

    assert!(intr_get_level() == IntrLevel::IntrOff);

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

/// Enables preemptive scheduling.
/// Thread system must have been previously enabled.
pub fn thread_system_start(kernel_page_manager: PageManager, init_elf: &[u8]) -> ! {
    assert!(intr_get_level() == IntrLevel::IntrOff);
    assert!(
        unsafe { THREAD_SYSTEM_INITIALIZED },
        "Cannot start threading without initializing the threading system."
    );

    // We must 'turn the kernel thread into a thread'.
    // This amounts to just making a TCB that will be in control of the kernel stack and will
    // never exit.
    // This thread also does not need to enter the `run_thread` function.
    // SAFETY: The kernel thread's stack will be set up by the context switch following.
    let tcb_kernel = unsafe { ThreadControlBlock::create_kernel_thread(kernel_page_manager) };

    let init_tcb = ThreadControlBlock::create(init_elf);

    // SAFETY: Interrupts must be disabled.
    unsafe {
        RUNNING_THREAD = Some(Box::new(tcb_kernel));
        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(Box::new(init_tcb));
    }

    // Enable preemptive scheduling.
    intr_enable();

    // Eventually, the scheduler may run the kernel thread again.
    // We may later replace this with code to clean up the kernel resources (`thread_exit` would not work).
    // For now we will just yield continually.
    loop {
        scheduler_yield();
    }

    // This function never returns.
}
