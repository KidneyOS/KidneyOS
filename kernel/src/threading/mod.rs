mod context_switch;
pub mod scheduling;
mod thread_control_block;
mod thread_functions;

use crate::{
    paging::{PageManager, PageManagerDefault},
    sync::{intr_enable, intr_get_level, IntrLevel},
};
use core::ptr::NonNull;
use alloc::boxed::Box;
use kidneyos_shared::{
    println,
    serial::outb,
};
use scheduling::{initialize_scheduler, scheduler_yield, SCHEDULER};
use thread_control_block::{ProcessControlBlock, ThreadControlBlock, Tid};

static mut RUNNING_THREAD: Option<Box<ThreadControlBlock>> = None;

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

/// Enables preemptive scheduling.
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
    let kernel_tcb = unsafe { ThreadControlBlock::new_kernel_thread(kernel_page_manager) };

    // Create the initial user program thread.
    let user_tcb = ProcessControlBlock::new(init_elf);

    // Create the idle thread.
    /* TODO: gets the following error when switched into in a context switch.
     * page fault with error code 0b101 occurred when trying to access 0x80148D00 from instruction at 0x80148D00
     */
    let idle_func_ptr = NonNull::new(idle_function as *mut u8).expect("idle function pointer is null!");
    let idle_tcb = ThreadControlBlock::new(idle_func_ptr, user_tcb.pid, PageManager::default());

    println!("(TID) kernel: {:?}, user: {:?}, idle: {:?}", kernel_tcb.tid, user_tcb.tid, idle_tcb.tid);

    // SAFETY: Interrupts must be disabled.
    unsafe {
        RUNNING_THREAD = Some(Box::new(kernel_tcb));

        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(Box::new(idle_tcb));
        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(Box::new(user_tcb));
    }

    // Enable preemptive scheduling.
    unsafe {
        outb(0x21, 0xfd);
        outb(0xa1, 0xff);
    }
    intr_enable(IntrLevel::IntrOn);

    // Eventually, the scheduler may run the kernel thread again.
    // We may later replace this with code to clean up the kernel resources (`thread_exit` would not work).
    // For now we will just yield continually.
    loop {
        scheduler_yield();
    }

    // This function never returns.
}

/// The function run by the idle thread.
/// Continually yields and should never die.
extern "C" fn idle_function() -> () {
    loop {
        println!("idle");
        scheduler_yield();
    }
}
