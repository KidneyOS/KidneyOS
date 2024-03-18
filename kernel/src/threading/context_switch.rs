use crate::threading::ThreadControlBlock;
use core::mem::align_of;

use alloc::boxed::Box;

use super::{scheduling::SCHEDULER, thread_control_block::ThreadStatus, RUNNING_THREAD};

/**
 * Public facing method to perform a context switch between two threads.
 *
 * # Safety
 * This function should only be called by methods within the Scheduler crate.
 * Interrupts must be disabled.
 */
pub unsafe fn switch_threads(switch_to: Box<ThreadControlBlock>) {
    let switch_from = Box::into_raw(RUNNING_THREAD.take().expect("Why is nothing running!?"));
    let switch_to = Box::into_raw(switch_to);

    // Ensure we are switching to a valid thread.
    if !matches!((*switch_to).status, ThreadStatus::Ready) {
        panic!("Cannot switch to a non-ready thread.");
    }

    // Ensure that the previous thread is no longer running.
    if matches!((*switch_from).status, ThreadStatus::Running) {
        panic!("The thread to switch out of must no longer be in the running state.");
    }

    let previous = context_switch(switch_from, switch_to);

    // We must mark this thread as running once again.
    (*switch_from).status = ThreadStatus::Running;

    // After threads have switched, we must update the scheduler and running thread.
    RUNNING_THREAD = Some(alloc::boxed::Box::from_raw(switch_from));
    SCHEDULER
        .as_mut()
        .expect("Scheduler not set up!")
        .push(Box::from_raw(previous));
}

#[macro_export]
macro_rules! save_registers {
    () => {
        // Saves the current thread's registers into it's context (on the stack).
        r#"
        push ebp
        mov ebp, esp    # Part of the calling convention, saving where this stack starts.
                        # We allocate no local variables.
        push ebx
        push esi
        push edi
        "#
    };
}

#[macro_export]
macro_rules! load_arguments {
    () => {
        // Loads two arguments from the stack into %eax and %edx.
        // * %eax = [%ebp + 0x8] is our first argument (switch_from).
        // * %edx = [%ebp + 0x12] is our second argument (switch_to).
        // These arguments are pointers to the TCB / Stack pointers.
        //
        // Note: We do not change the value of %eax as this will be our return value.
        r#"
            mov eax, [ebp + 0x8]
            mov edx, [ebp + 0xc]
        "#
    };
}

#[macro_export]
macro_rules! switch_stacks {
    () => {
        // Switches the current stack pointer.
        // Both %eax and %edx are *TCB = **stack and thus must be dereferenced once to get the stack pointer.
        r#"
            mov [eax], esp
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
 * Performs a context switch between two threads.
 *
 * Must save the Callee's registers and restore the next's registers.
 *
 * The caller saved registers are: %eax, %ecx, and %edx.
 * So we may use them freely.
 * All others must be saved as part of the context switch.
 *
 * Parameters are pushed to the stack the opposite order they are defined.
 * The last is pushed to the stack first (higher address), and the first is pushed last (lower address).
 * The caller is responisble to remove these from the stack.
 *
 * Our return value will need to be placed into the %eax register.
 *
 */
#[naked]
#[no_mangle]
unsafe extern "C" fn context_switch(
    _switch_from: *mut ThreadControlBlock,
    _switch_to: *mut ThreadControlBlock,
) -> *mut ThreadControlBlock {
    // Our function arguments are placed on the stack Right to Left.
    core::arch::asm!(
        save_registers!(),
        load_arguments!(), // Required manually since this is a naked function.
        switch_stacks!(),
        restore_registers!(),
        r#"
            ret
        "#, // Required manually since this is a naked function.
        options(noreturn)
    )
}
