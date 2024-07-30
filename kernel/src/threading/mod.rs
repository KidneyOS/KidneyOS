mod context_switch;
pub mod scheduling;
pub mod thread_management;
mod thread_control_block;
mod thread_functions;

use crate::{
    paging::PageManager,
    sync::intr::{intr_enable, intr_get_level, IntrLevel},
};
use kidneyos_shared::println;
use scheduling::{initialize_scheduler, scheduler_yield, SCHEDULER};
use thread_control_block::{ThreadControlBlock, Tid};
use thread_management::{initialize_thread_manager, THREAD_MANAGER};

// Invalid until thread system intialized.
pub static mut RUNNING_THREAD_TID: Tid = 0;

/// To be called before any other thread functions.
/// To be called with interrupts disabled.
static mut THREAD_SYSTEM_INITIALIZED: bool = false;
pub fn thread_system_initialization() {
    println!("Initializing Thread System...");

    assert!(intr_get_level() == IntrLevel::IntrOff);

    // Initialize the TID lock.

    // Initialize the scheduler.
    initialize_scheduler();

    // Initialize thread manager.
    initialize_thread_manager();

    // Create Idle thread.

    // SAFETY: Interrupts must be disabled.
    unsafe {
        THREAD_SYSTEM_INITIALIZED = true;
    }
    println!("Finished Thread System initialization. Ready to start threading.");
}

const INIT_A: &[u8] = include_bytes!("../../programs/loop/loop").as_slice();
const INIT_B: &[u8] = include_bytes!("../../programs/loop/loop").as_slice();
const INIT_C: &[u8] = include_bytes!("../../programs/loop/loop").as_slice();

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
    
    let init_tcb = ThreadControlBlock::create(init_elf);
    let init_tcb_a = ThreadControlBlock::create(INIT_A);
    let init_tcb_b = ThreadControlBlock::create(INIT_B);
    let init_tcb_c = ThreadControlBlock::create(INIT_C);

    unsafe {
        let tm = 
            THREAD_MANAGER
                .as_mut()
                .expect("No Thread Manager set up!");
        
        let tcb_kernel = ThreadControlBlock::create_kernel_thread(kernel_page_manager);

    // SAFETY: Interrupts must be disabled.

        RUNNING_THREAD_TID = tm.add(tcb_kernel);

        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(
                tm.add(init_tcb_a)
            );
        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(
                tm.add(init_tcb_b)
            );
        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(
                tm.add(init_tcb_c)
            );
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
