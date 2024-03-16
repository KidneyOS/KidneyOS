use crate::threading::ThreadControlBlock;

use crate::println;
use alloc::boxed::Box;

use super::{scheduling::SCHEDULER, RUNNING_THREAD, thread_control_block::ThreadStatus};

/**
 * Public facing method to perform a context switch between two threads.
 *
 * SAFETY: This function should only be called by methods within the Scheduler crate.
 */
pub unsafe fn switch_threads(mut switch_to: ThreadControlBlock) {
    let switch_from_box = RUNNING_THREAD.take().expect("Why is nothing running!?");
    let switch_from = Box::into_raw(switch_from_box);
    // switch_from.status = ThreadStatus::Ready;

    println!(
        "FROM {:?}\n  TO {:?}\n",
        switch_from,
        core::ptr::addr_of_mut!(switch_to),
    );

    // TODO:
    // Safety checks needed.
    let previous = context_switch(
        // core::ptr::addr_of_mut!(switch_from),
        switch_from,
        core::ptr::addr_of_mut!(switch_to),
    );

    // switch_from.status = ThreadStatus::Running;

    // After threads have switched, we must update the scheduler and running thread.
    RUNNING_THREAD = Some(alloc::boxed::Box::from_raw(switch_from));
    SCHEDULER
        .as_mut()
        .expect("Scheduler not set up!")
        .push(core::ptr::read(previous.cast_const()));
}

/**
 * See: `SwitchThreadsContext` for ordering details.
 */

#[macro_export]
macro_rules! load_arguments {
    () => {
        // Loads two arguments from the stack into %eax and %edx.
        // Note, `call` should be used just before this.
        // So, at this point:
        // * [%esp] is an instruction pointer.
        // * [%esp + 0x4] is our first argument (switch_from).
        // * [%esp + 0x8] is our second argument (switch_to).
        // These arguments are pointers to the TCB / Stack pointers.
        // These registers must be preserved as `prepare_thread` expects them to stay as such (and functions using it's return value).
        // TODO: Move ESP (add esp, 0x8) to remove these arguments?
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
        // Requires that %eax holds the pointer to the stack pointer of the previous thread
        // and that %edx holds the pointer to the stack pointer of the new thread.
        // We use %ecx to deal with this level of indirection.
        r#"
            mov ecx, [eax]
            mov [ecx], esp
            mov esp, [edx]
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
unsafe fn context_switch(
    _switch_from: *mut ThreadControlBlock,
    _switch_to: *mut ThreadControlBlock,
) -> *mut ThreadControlBlock {
    // Our function arguments are placed on the stack Right to Left.
    core::arch::asm!(
        load_arguments!(), // Required manually since this is a naked function.
        save_registers!(),
        switch_stacks!(),
        restore_registers!(),
        r#"
            ret
        "#, // Required manually since this is a naked function.
        options(noreturn)
    )
}
