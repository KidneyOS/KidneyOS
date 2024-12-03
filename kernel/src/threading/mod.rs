mod context_switch;
pub mod process;
pub mod process_functions;
pub mod scheduling;
pub mod thread_control_block;
pub mod thread_functions;
pub mod thread_sleep;

use crate::drivers::ata::ata_core::ide_init;
use crate::rush::rush_core::rush_loop;
use crate::sync::mutex::Mutex;
use crate::system::unwrap_system;
use crate::threading::scheduling::{scheduler_yield_and_continue, Scheduler};
use crate::user_program::elf::Elf;
use crate::{
    interrupts::{intr_enable, intr_get_level, IntrLevel},
    paging::PageManager,
    threading::scheduling::create_scheduler,
};
use alloc::boxed::Box;
use core::ptr::NonNull;
use thread_control_block::ThreadControlBlock;

pub struct ThreadState {
    pub running_thread: Mutex<Option<Box<ThreadControlBlock>>>,
    pub scheduler: Mutex<Box<dyn Send + Scheduler>>,
}

pub fn create_thread_state() -> ThreadState {
    assert_eq!(intr_get_level(), IntrLevel::IntrOff);

    // Initialize the scheduler.
    let scheduler = Mutex::new(create_scheduler());

    // SAFETY: Interrupts must be disabled.

    ThreadState {
        running_thread: Mutex::new(None), // Drop Option<> and set this to the IDLE thread?
        scheduler,
    }
}

/// Thread system must have been previously enabled.
pub fn thread_system_start(kernel_page_manager: PageManager, init_elf: &[u8]) -> ! {
    assert_eq!(intr_get_level(), IntrLevel::IntrOff);
    let system = unwrap_system();
    // We must 'turn the kernel thread into a thread'.
    // This amounts to just making a TCB that will be in control of the kernel stack and will
    // never exit.
    // This thread also does not need to enter the `run_thread` function.
    // SAFETY: The kernel thread's stack will be set up by the context switch following.
    // SAFETY: The kernel thread is allocated a "fake" PCB with pid 0.
    let kernel_tcb = ThreadControlBlock::new_kernel_thread(kernel_page_manager, &system.process);

    // Create the initial user program thread.
    let elf = Elf::parse_bytes(init_elf).expect("failed to parse provided elf file");

    // Create the initial user program thread.
    let user_tcb = ThreadControlBlock::new_from_elf(elf, &system.process)
        .expect("Failed to parse Elf for initial program.");

    // Create the IDE initialization thread
    let ide_addr = NonNull::new(ide_init as *const () as *mut u8).unwrap();
    let ide_tcb = ThreadControlBlock::new_with_setup(ide_addr, true, &system.process);

    // Create RUSH thread
    let rush_addr = NonNull::new(rush_loop as *const () as *mut u8).unwrap();
    let rush_tcb = ThreadControlBlock::new_with_setup(rush_addr, true, &system.process);

    // SAFETY: Interrupts must be disabled.
    *system.threads.running_thread.lock() = Some(Box::new(kernel_tcb));
    let mut scheduler = system.threads.scheduler.lock();
    scheduler.push(Box::new(ide_tcb));
    scheduler.push(Box::new(user_tcb));
    scheduler.push(Box::new(rush_tcb));
    drop(scheduler);

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
