use super::{
    scheduling::SCHEDULER,
    thread_control_block::{ThreadControlBlock, ThreadStatus},
    RUNNING_THREAD,
};
use crate::sync::intr::intr_enable;
use core::arch::asm;
use kidneyos_shared::{
    global_descriptor_table::{USER_CODE_SELECTOR, USER_DATA_SELECTOR},
    task_state_segment::TASK_STATE_SEGMENT,
};

use alloc::boxed::Box;

/// TODO: Thread arguments: Usually a void ptr, but Rust won't like that...
/// No arguments allowed for now.
///
/// A function that may be used for thread creation.
pub type ThreadFunction = unsafe extern "C" fn() -> ();

/// A function to safely close a thread.
#[allow(unused)]
const fn exit_thread() -> ! {
    // TODO: Need to reap TCB, remove from scheduling.

    // Relinquish CPU to another thread.
    // TODO:
    panic!("Thread exited incorrectly.");
}

/// A wrapper function to execute a thread's true function.
unsafe extern "C" fn run_thread(
    switched_from: *mut ThreadControlBlock,
    switched_to: *mut ThreadControlBlock,
) -> ! {
    let mut switched_to = Box::from_raw(switched_to);

    // We assume that switched_from had it's status changed already.
    // We must only mark this thread as running.
    switched_to.status = ThreadStatus::Running;

    TASK_STATE_SEGMENT.esp0 = switched_to.kernel_stack.as_ptr() as u32;

    let ThreadControlBlock { eip, esp, .. } = *switched_to;

    // Reschedule our threads.
    RUNNING_THREAD = Some(switched_to);
    SCHEDULER
        .as_mut()
        .expect("Scheduler not set up!")
        .push(Box::from_raw(switched_from));

    // Our scheduler will operate without interrupts.
    // Every new thread should start with them enabled.
    intr_enable();

    // https://wiki.osdev.org/Getting_to_Ring_3#iret_method
    // https://web.archive.org/web/20160326062442/http://jamesmolloy.co.uk/tutorial_html/10.-User%20Mode.html

    asm!(
        "
        mov ds, {data_sel:x}
        mov es, {data_sel:x}
        mov fs, {data_sel:x}
        mov gs, {data_sel:x} // SS and CS are handled by iret

        // Set up the stack frame iret expects.
        push {data_sel:e} // stack segment
        push {esp} // esp
        pushfd // eflags
        push {code_sel} // code segment
        push {eip} // eip
        iretd
        ",
        data_sel = in(reg) USER_DATA_SELECTOR,
        esp = in(reg) esp.as_ptr(),
        code_sel = const USER_CODE_SELECTOR,
        eip = in(reg) eip.as_ptr(),
        options(noreturn),
    );
}

/// This function is used to clean up a thread's arguments and call into `run_thread`.
#[naked]
unsafe extern "C" fn prepare_thread() {
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
