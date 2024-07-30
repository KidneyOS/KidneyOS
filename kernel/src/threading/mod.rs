mod context_switch;
pub mod scheduling;
pub mod thread_control_block;
pub mod thread_functions;

use crate::{
    paging::PageManager,
    sync::intr::{intr_enable, intr_get_level, IntrLevel},
    threading::scheduling::{initialize_scheduler, scheduler_yield_and_continue, SCHEDULER},
};
use alloc::boxed::Box;
use thread_control_block::{ProcessControlBlock, ThreadControlBlock, Tid};

pub static mut RUNNING_THREAD: Option<Box<ThreadControlBlock>> = None;

/// To be called before any other thread functions.
/// To be called with interrupts disabled.
static mut THREAD_SYSTEM_INITIALIZED: bool = false;
pub fn thread_system_initialization() {
    assert_eq!(intr_get_level(), IntrLevel::IntrOff);

    // Initialize the scheduler.
    initialize_scheduler();

    // SAFETY: Interrupts must be disabled.
    unsafe {
        THREAD_SYSTEM_INITIALIZED = true;
    }
}

/// Thread system must have been previously enabled.
pub fn thread_system_start(kernel_page_manager: PageManager, init_elf: &[u8]) -> ! {
    assert_eq!(intr_get_level(), IntrLevel::IntrOff);
    assert!(
        unsafe { THREAD_SYSTEM_INITIALIZED },
        "Cannot start threading without initializing the threading system."
    );

    // We must 'turn the kernel thread into a thread'.
    // This amounts to just making a TCB that will be in control of the kernel stack and will
    // never exit.
    // This thread also does not need to enter the `run_thread` function.
    // SAFETY: The kernel thread's stack will be set up by the context switch following.
    // SAFETY: The kernel thread is allocated a "fake" PCB with pid 0.
    let kernel_tcb = ThreadControlBlock::new_kernel_thread(kernel_page_manager);

    // Create the initial user program thread.
    let user_tcb = ProcessControlBlock::new(init_elf);

    // SAFETY: Interrupts must be disabled.
    unsafe {
        RUNNING_THREAD = Some(Box::new(kernel_tcb));

        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(Box::new(user_tcb));
    }

    intr_enable();

    // Eventually, the scheduler may run the kernel thread again.
    // We may later replace this with code to clean up the kernel resources.
    // For now, we will act as the idle thread.
    idle_function();

    // This function never returns.
}

/// The function run by the idle thread.
/// Continually yields and should never die.
extern "C" fn idle_function() -> ! {
    loop {
        scheduler_yield_and_continue();
    }
}
