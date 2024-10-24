use super::scheduling::{scheduler_yield_and_block, scheduler_yield_and_continue};
use super::thread_control_block::{ThreadControlBlock, ThreadStatus};
use super::thread_sleep::thread_wakeup;
use crate::system::unwrap_system_mut;
use crate::{
    interrupts::{intr_disable, intr_enable},
    threading::scheduling::scheduler_yield_and_die,
};
use alloc::boxed::Box;
use core::arch::asm;
use kidneyos_shared::{
    global_descriptor_table::{USER_CODE_SELECTOR, USER_DATA_SELECTOR},
    task_state_segment::TASK_STATE_SEGMENT,
};

/// TODO: Thread arguments: Usually a void ptr, but Rust won't like that...
/// No arguments allowed for now.
///
/// A function that may be used for thread creation.
/// The return value will be the exit code of this thread.
pub type ThreadFunction = unsafe extern "C" fn() -> i32;

/// A function to safely close the current thread.
/// This is safe to call at any point in a threads runtime.
#[allow(unused)]
pub fn exit_thread(exit_code: i32) -> ! {
    // We will never return here so do not need to re-enable interrupts from here.
    intr_disable();

    let system = unsafe { unwrap_system_mut() };
    let process_table = &system.process.table;
    let pcb = process_table
        .get(system.threads.running_thread.as_ref().unwrap().pid)
        .unwrap();

    // Currently we just immediately yield the thread, in the future this could be set up to
    // Put the thread on the sleep queue (although with the current sleep queue implementation
    // that would not provide any performence benifits).
    while pcb.wait_list.is_empty() {
        // TODO: This puts the thread's status as 'Ready', however by now it is a zombie thread.
        scheduler_yield_and_continue()
    }

    pcb.wait_list.iter().for_each(|pid| thread_wakeup(*pid));

    // Get the current thread.
    // SAFETY: Interrupts must be off.
    unsafe {
        let threads = &mut unwrap_system_mut().threads;
        let mut current_thread = threads
            .running_thread
            .take()
            .expect("Why is nothing running!?");
        current_thread.set_exit_code(exit_code);

        // Replace and yield.
        threads.running_thread = Some(current_thread);
        scheduler_yield_and_die();
    }
}

/// A wrapper function to execute a thread's true function.
unsafe extern "C" fn run_thread(
    switched_from: *mut ThreadControlBlock,
    switched_to: *mut ThreadControlBlock,
) -> ! {
    let threads = &mut unwrap_system_mut().threads;
    let mut switched_to = Box::from_raw(switched_to);

    // We assume that switched_from had its status changed already.
    // We must only mark this thread as running.
    switched_to.status = ThreadStatus::Running;

    TASK_STATE_SEGMENT.esp0 = switched_to.kernel_stack.as_ptr() as u32;

    let ThreadControlBlock { eip, esp, pid, .. } = *switched_to;

    // Reschedule our threads.
    threads.running_thread = Some(switched_to);

    let mut switched_from = Box::from_raw(switched_from);

    if switched_from.status == ThreadStatus::Dying {
        switched_from.reap();

        // Page manager must be loaded to be dropped.
        switched_from.page_manager.load();
        drop(switched_from);
        threads.running_thread.as_ref().unwrap().page_manager.load();
    } else {
        threads.scheduler.push(switched_from);
    }

    // Our scheduler will operate without interrupts.
    // Every new thread should start with them enabled.
    intr_enable();

    // Kernel threads have no associated PCB, denoted by its PID being 0
    if pid == 0 {
        let entry_function = eip.as_ptr() as *const ThreadFunction;
        let exit_code = (*entry_function)();

        // Safely exit the thread.
        exit_thread(exit_code);
    } else {
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
            push {esp}
            pushfd // eflags
            push {code_sel} // code segment
            push {eip}
            iretd
            ",
            data_sel = in(reg) USER_DATA_SELECTOR,
            esp = in(reg) esp.as_ptr(),
            code_sel = const USER_CODE_SELECTOR,
            eip = in(reg) eip.as_ptr(),
            options(noreturn),
        )
    }
}

#[allow(unused)]
#[repr(C, packed)]
pub struct PrepareThreadContext {
    entry_function: *const u8,
}

impl PrepareThreadContext {
    pub fn new(entry_function: *const u8) -> Self {
        Self { entry_function }
    }
}

/// This function is used to clean up a thread's arguments and call into `run_thread`.
#[naked]
unsafe extern "C" fn prepare_thread() -> i32 {
    // Since this function is only to be called from the `context_switch` function, we expect
    // That %eax and %edx contain the arguments passed to it.
    // Further, the entry function pointer is at a known position on the stack.
    // We move this into a register and call the run thread function.
    asm!(
        r#"
            # push [esp] # Already in place on stack.
            push edx
            push eax
            call {}
            hlt     # Never return to here.
        "#,
        sym run_thread,
        options(noreturn)
    )
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
