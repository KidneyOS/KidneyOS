use super::scheduling::SCHEDULER;
use super::thread_control_block::ThreadControlBlock;
use super::RUNNING_THREAD;

use crate::alloc::boxed::Box;

// TODO: Thread arguments: Usually a void ptr, but Rust won't like that...
// No arguments allowed for now.
/**
 * A function that may be used for thread creation.
 */
pub type ThreadFunction = fn() -> ();

/**
 * A function to safely close a thread.
 */
const fn exit_thread() {
    // TODO: Need to reap TCB, remove from scheduling.

    // Relinquish CPU to another thread.
    // TODO:
    panic!("Thread Exited incorrectly.");
}

#[repr(C, packed)]
pub struct RunThreadContext {
    padding: [u8; 4], // Oddly, this seems required.
    switched_from: *const ThreadControlBlock,
    switched_to: *const ThreadControlBlock,
    entry_function_pointer: *const ThreadFunction,
    eip: usize, // Should always be NULL.
}

impl RunThreadContext {
    pub fn create(entry_function: ThreadFunction) -> Self {
        Self {
            padding: [0, 0, 0, 0],
            switched_from: core::ptr::null(),
            switched_to: core::ptr::null(), // These will be provided values within `prepare_thread`.
            entry_function_pointer: entry_function as *const ThreadFunction,
            eip: 0,
        }
    }
}

/**
 * A wrapper function to execute a thread's true function.
 */
#[no_mangle]
unsafe fn run_thread(
    switched_from: *mut ThreadControlBlock,
    switched_to: *mut ThreadControlBlock,
    entry_function: ThreadFunction,
) {
    // TODO: Safety checks.

    // Reschedule our threads.
    RUNNING_THREAD = Some(alloc::boxed::Box::from_raw(switched_to));
    SCHEDULER
        .as_mut()
        .expect("Scheduler not set up!")
        .push(Box::from_raw(switched_from));

    // Our scheduler will operate without interrupts.
    // Every new thread should start with them enabled.
    // interrupts_enable();

    // Run the thread.
    entry_function();

    // Safely exit the thread.
    exit_thread();
}

#[repr(C, packed)]
pub struct PrepareThreadContext {
    eip: *const ThreadFunction, // Should always be set to &run_thread.
}

impl PrepareThreadContext {
    pub fn create() -> Self {
        Self {
            eip: run_thread as *const ThreadFunction,
        }
    }
}

/**
 * This function is used to clean up a thread's arguments and call into `run_thread`.
 */
#[naked]
#[no_mangle]
unsafe extern "C" fn prepare_thread() {
    // We must place the TCB pointers left from the context switch onto the stack for `run_thread`.
    // Since this function is only to be called from the `context_switch` function, we expect
    // That %eax and %edx contain the arguments passed to it.
    // These are pushed onto the stack for `run_thread`.
    // No `call` instruction is used to get here, only `ret`.
    // So there should be one less instruction pointer value on our stack.
    core::arch::asm!(
        r#"
            mov [esp + 0x4], eax
            mov [esp + 0x8], edx
            ret
        "#,
        options(noreturn)
    );
}

/**
 * The context for a use within context_switch.
 */
#[repr(C, packed)]
pub struct SwitchThreadsContext {
    edi: usize,                 // Destination index.
    esi: usize,                 // Source index.
    ebx: usize,                 // Base (for memory access).
    ebp: usize,                 // Stack base pointer.
    eip: *const ThreadFunction, // Instruction pointer.
}

impl SwitchThreadsContext {
    pub fn empty_context() -> Self {
        Self {
            edi: 0,
            esi: 0,
            ebx: 0,
            ebp: 0,
            eip: core::ptr::null(),
        }
    }

    pub fn create() -> Self {
        Self {
            edi: 0,
            esi: 0,
            ebx: 0,
            ebp: 0,
            eip: prepare_thread as *const ThreadFunction,
        }
    }
}
