
// TODO: Thread arguments: Usually a void ptr, but Rust won't like that...
// No arguments allowed for now.
/**
 * A function that may be used for thread creation.
 */
pub type ThreadFunction = fn() -> ();

/**
 * A function to safely close a thread.
 */
fn thread_exit() {

    // TODO: Need to reap TCB, remove from scheduling.

    // Relinquish CPU to another thread.
    // TODO:
    loop {}

}

#[repr(C, packed)]
pub struct RunThreadContext {
    pub eip: usize, // Should always be NULL.
    pub entry_function_pointer: usize
}

/**
 * A wrapper function to execute a thread's true function.
 */
pub fn run_thread(function: ThreadFunction) {

    // TODO: Safety checks.

    // Our scheduler will operate without interrupts.
    // Every new thread should start with them enabled.
    // interrupts_enable();

    // Run the thread.
    function();

    // Safely exit the thread.
    thread_exit();

}

#[repr(C, packed)]
pub struct PrepareThreadContext {
    pub eip: usize, // Should always be set to &run_thread.
}

/**
 * This function is used to clean up a thread's arguments and call into `run_thread`.
 */
#[naked]
pub unsafe fn prepare_thread() {

    // This is going to be uncessary (potentially) until we add an argument for the thread's entry function.
    core::arch::asm!(
        r#"
            ret
        "#,
        options(noreturn)
    );

}
