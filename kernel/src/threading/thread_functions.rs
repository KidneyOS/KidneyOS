use crate::{
    sync::{intr_disable, intr_enable},
    threading::scheduling::scheduler_yield_and_die,
};

use super::{
    scheduling::SCHEDULER,
    thread_control_block::{ThreadControlBlock, ThreadStatus},
    RUNNING_THREAD,
};

use alloc::boxed::Box;

/// TODO: Thread arguments: Usually a void ptr, but Rust won't like that...
/// No arguments allowed for now.
///
/// A function that may be used for thread creation.
/// The return value will be the exit code of this thread.
pub type ThreadFunction = unsafe extern "C" fn() -> Option<i32>;

/// A function to safely close the current thread.
/// This is safe to call at any point in a threads runtime.
pub fn exit_thread(exit_code: i32) -> ! {
    // Disable interrupts.
    // TODO: Does this cause issues with the counter? Do we need a 'force enable' or 'reset' for this?
    intr_disable();

    // Get the current thread.
    // SAFETY: Interrupts must be off.
    unsafe {
        let mut current_thread = RUNNING_THREAD.take().expect("Why is nothing running!?");
        current_thread.set_exit_code(exit_code);

        // Replace and yield.
        RUNNING_THREAD = Some(current_thread);
        scheduler_yield_and_die();
    }
}

/// A wrapper function to execute a thread's true function.
unsafe extern "C" fn run_thread(
    switched_from: *mut ThreadControlBlock,
    switched_to: *mut ThreadControlBlock,
    entry_function: ThreadFunction,
) -> ! {
    // We assume that switched_from had it's status changed already.
    // We must only mark this thread as running.
    (*switched_to).status = ThreadStatus::Running;

    // Reschedule our threads.
    RUNNING_THREAD = Some(Box::from_raw(switched_to));

    let mut switched_from = Box::from_raw(switched_from);
    if switched_from.status == ThreadStatus::Dying {
        switched_from.reap();
        drop(switched_from);
    } else {
        SCHEDULER
            .as_mut()
            .expect("Scheduler not set up!")
            .push(switched_from);
    }

    // Our scheduler will operate without interrupts.
    // Every new thread should start with them enabled.
    intr_enable();

    // Run the thread.
    let exit_code = entry_function().unwrap_or_default();

    // Safely exit the thread.
    exit_thread(exit_code);
}

#[repr(C, packed)]
pub struct PrepareThreadContext {
    entry_function: ThreadFunction,
}

impl PrepareThreadContext {
    pub fn new(entry_function: ThreadFunction) -> Self {
        Self { entry_function }
    }
}

/// This function is used to clean up a thread's arguments and call into `run_thread`.
#[naked]
unsafe extern "C" fn prepare_thread() -> Option<i32> {
    // Since this function is only to be called from the `context_switch` function, we expect
    // That %eax and %edx contain the arguments passed to it.
    // Further, the entry function pointer is at a known position on the stack.
    // We move this into a register and call the run thread function.
    core::arch::asm!(
        r#"
            # push [esp] # Already in place on stack.
            push edx
            push eax
            call {}
            hlt     # Never return to here.
        "#,
        sym run_thread,
        options(noreturn)
    );
}

/// The context for a use within context_switch.
#[repr(C, packed)]
pub struct SwitchThreadsContext {
    edi: usize,          // Destination index.
    esi: usize,          // Source index.
    ebx: usize,          // Base (for memory access).
    ebp: usize,          // Stack base pointer.
    eip: ThreadFunction, // Instruction pointer (determines where to jump after the context switch).
}

impl SwitchThreadsContext {
    pub fn new() -> Self {
        Self {
            edi: 0,
            esi: 0,
            ebx: 0,
            ebp: 0,
            eip: prepare_thread,
        }
    }
}
