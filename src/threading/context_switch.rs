
use crate::threading::ThreadControlBlock;

/**
 * Public facing method to perform a context switch between two threads.
 */
pub fn switch_threads(switch_from: ThreadControlBlock, switch_to: ThreadControlBlock) {

    // TODO:
    // switch_from should not need to be passed in (will get covered by scheduling)
    // Safety checks needed.

    unsafe {
        context_switch(
            switch_from.stack_pointer.as_ptr() as *mut usize,
            switch_to.stack_pointer.as_ptr() as usize
        );
    }

    // Here is where we need to push to the scheduler and set the new running thread.

}

/**
 * See: `SwitchThreadsContext` for ordering details.
 */

#[macro_export]
macro_rules! load_arguments {
    () => {
        // Loads two arguments from the stack into %eax and %edx.
        // Note, `call` should be used just before this.
        // So, [%esp] is an instruction pointer, [%esp + 0x4] is our first argument, and [%esp + 0x8] is our second argument.
        r#"
            mov eax, [esp + 0x4]
            mov edx, [esp + 0x8]
        "#
    };
}

#[macro_export]
macro_rules! save_registers {
    () => {
        // Saves the current thread's registers into it's context (on the stack).
        r#"
            push ebp
            push ebx
            push esi
            push edi
        "#
    };
}

#[macro_export]
macro_rules! switch_stacks {
    () => {
        // Switches the current stack pointer.
        // Requires that %eax hold a pointer to the current stack.
        //          and that %edx holds the value of the next stack.
        r#"
            mov [eax], esp
            mov esp, edx
        "#
    };
}

#[macro_export]
macro_rules! restore_registers {
    () => {
        // Pops from the current stack to restore the thread's context.
        r#"
            pop edi
            pop esi
            pop ebx
            pop ebp
        "#
    };
}

/**
 * The usize here represents the pointer to the struct itself.
 * That is, it's value is an address.
 * Effectively the signature:
 *      fn context_switch(context **previous, context *next);
 *
 * Must save the Callee's registers and restore the next's registers.
 */
#[naked]
unsafe fn context_switch(_previous_stack_pointer: *mut usize, _next_stack_pointer: usize) {

    // Our function arguments are placed on the stack Right to Left.
    core::arch::asm!(
        load_arguments!(),  // Required manually since this is a naked function.
        save_registers!(),
        switch_stacks!(),
        restore_registers!(),
        r#"
            ret
        "#,                 // Required manually since this is a naked function.
        options(noreturn)
    )

}
