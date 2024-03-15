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
    eip: usize, // Should always be NULL.
    entry_function_pointer: usize,
}

impl RunThreadContext {
    pub fn create(entry_function: ThreadFunction) -> Self {
        Self {
            eip: 0,
            entry_function_pointer: entry_function as usize,
        }
    }
}

/**
 * A wrapper function to execute a thread's true function.
 */
fn run_thread(function: ThreadFunction) {
    // TODO: Safety checks.

    // Our scheduler will operate without interrupts.
    // Every new thread should start with them enabled.
    // interrupts_enable();

    // Run the thread.
    function();

    // Safely exit the thread.
    exit_thread();
}

#[repr(C, packed)]
pub struct PrepareThreadContext {
    eip: usize, // Should always be set to &run_thread.
}

impl PrepareThreadContext {
    pub fn create() -> Self {
        Self {
            eip: run_thread as usize,
        }
    }
}

/**
 * This function is used to clean up a thread's arguments and call into `run_thread`.
 */
#[naked]
unsafe fn prepare_thread() {
    // This is going to be uncessary (potentially) until we add an argument for the thread's entry function.
    core::arch::asm!(
        r#"
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
