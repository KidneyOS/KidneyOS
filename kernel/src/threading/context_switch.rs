use crate::{sync::intr::{intr_get_level, IntrLevel}, threading::RUNNING_THREAD_TID};
use core::mem::offset_of;
use alloc::boxed::Box;
use kidneyos_shared::println;

use super::{
    scheduling::SCHEDULER,
    thread_management::THREAD_MANAGER,
    thread_control_block::{ThreadControlBlock, ThreadStatus, Tid},
};

/// Public facing method to perform a context switch between two threads.
/// # Safety
/// This function should only be called by methods within the Scheduler crate.
/// Interrupts must be disabled.
pub unsafe fn switch_threads(
    status_for_current_thread: ThreadStatus,
    switch_to: Tid,
) {
    assert_eq!(intr_get_level(), IntrLevel::IntrOff);

    let tm = THREAD_MANAGER
                                            .as_mut()
                                            .expect("No Thread Manager set up!");

    let switch_from = Box::into_raw(tm.get(RUNNING_THREAD_TID));
    let switch_to = Box::into_raw(tm.get(switch_to));

    // Ensure we are switching to a valid thread.
    assert!(
        (*switch_to).status == ThreadStatus::Ready,
        "Cannot switch to a non-ready thread."
    );

    // Ensure that the previous thread is running.
    assert!(
        (*switch_from).status == ThreadStatus::Running,
        "The thread to switch out of must be in the running state."
    );

    // Update the status of the current thread.
    (*switch_from).status = status_for_current_thread;

    let page_manager = &(*switch_to).page_manager;
    page_manager.load();
    println!("{} -> {}", (*switch_from).tid, (*switch_to).tid);

    let previous = Box::from_raw(context_switch(switch_from, switch_to));
    println!("{} -> {}", (*switch_from).tid, previous.tid);

    // We must mark this thread as running once again.
    (*switch_from).status = ThreadStatus::Running;

    // After threads have switched, we must update the scheduler and running thread.

    RUNNING_THREAD_TID = tm.set(Box::from_raw(switch_from));
    SCHEDULER
        .as_mut()
        .expect("Scheduler not set up!")
        .push(
            tm.set(previous)
        );
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
            mov [eax + {offset}], esp
            mov esp, [edx + {offset}]
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

/// Performs a context switch between two threads.
///
/// Must save the callee's registers and restore the next's registers.
///
/// The caller saved registers are: %eax, %ecx, and %edx.
/// So we may use them freely.
/// All others must be saved as part of the context switch.
///
/// Parameters are pushed to the stack the opposite order they are defined.
/// The last is pushed to the stack first (higher address), and the first is pushed last (lower address).
/// The caller is responsible to remove these from the stack.
///
/// Our return value will need to be placed into the %eax register.
#[naked]
pub unsafe extern "C" fn context_switch(
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
        offset = const offset_of!(ThreadControlBlock, kernel_stack_pointer),
        options(noreturn)
    )
}
