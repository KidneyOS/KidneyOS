use crate::block::block_core::BlockManager;
use crate::drivers::input::input_core::InputBuffer;
use crate::sync::mutex::Mutex;
use crate::threading::process::{Pid, ProcessState, Tid};
use crate::threading::thread_control_block::{ProcessControlBlock, ThreadControlBlock};
use crate::threading::ThreadState;

// Synchronizing this primitive in a safe way is hard.
pub struct SystemState {
    pub threads: ThreadState,
    pub process: ProcessState,

    pub block_manager: BlockManager,
    pub input_buffer: Mutex<InputBuffer>,
}

pub static mut SYSTEM: Option<SystemState> = None;

// SAFETY: SYSTEM references cannot be accessed simultaneously on different threads.
#[allow(dead_code)]
pub unsafe fn unwrap_system() -> &'static SystemState {
    SYSTEM.as_ref().expect("System not initialized.")
}

// SAFETY: SYSTEM references cannot be accessed simultaneously on different threads.
pub unsafe fn unwrap_system_mut() -> &'static mut SystemState {
    SYSTEM.as_mut().expect("System not initialized.")
}

/// Get reference to running process (panicks if no process is running)
///
/// # Safety
///
/// SYSTEM/process references cannot be accessed simultaneously on different threads.
pub unsafe fn running_process() -> &'static ProcessControlBlock {
    let system = unwrap_system();
    let pid = system.threads.running_thread.as_ref().unwrap().pid;
    system.process.table.get(pid).unwrap()
}

/// Get mutable reference to running process (panicks if no process is running)
///
/// # Safety
///
/// SYSTEM/process references cannot be accessed simultaneously on different threads.
pub unsafe fn running_process_mut() -> &'static mut ProcessControlBlock {
    let system = unwrap_system_mut();
    let pid = system.threads.running_thread.as_ref().unwrap().pid;
    system.process.table.get_mut(pid).unwrap()
}

pub fn running_thread_pid() -> Pid {
    let tcb = unsafe {
        unwrap_system()
            .threads
            .running_thread
            .as_ref()
            .expect("Why is nothing running?")
            .as_ref()
    };
    tcb.pid
}

pub fn running_thread_ppid() -> Pid {
    let tcb = unsafe {
        unwrap_system()
            .threads
            .running_thread
            .as_ref()
            .expect("Why is nothing running?")
            .as_ref()
    };
    let process_table = unsafe { &unwrap_system().process.table };
    let pcb = process_table.get(tcb.pid).unwrap();
    pcb.ppid
}

pub fn running_thread() -> &'static ThreadControlBlock {
    let tcb = unsafe {
        unwrap_system()
            .threads
            .running_thread
            .as_ref()
            .unwrap()
            .as_ref()
    };
    tcb
}

#[allow(dead_code)]
pub fn running_thread_mut() -> &'static mut ThreadControlBlock {
    let tcb = unsafe {
        unwrap_system_mut()
            .threads
            .running_thread
            .as_mut()
            .unwrap()
            .as_mut()
    };
    tcb
}

pub fn running_thread_tid() -> Tid {
    let tcb = running_thread();
    tcb.tid
}
