use crate::block::block_core::BlockManager;
use crate::threading::process::{ProcessState, Pid};
use crate::threading::ThreadState;

// Synchronizing this primitive in a safe way is hard.
pub struct SystemState {
    pub threads: ThreadState,
    pub process: ProcessState,

    pub block_manager: BlockManager,
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