use crate::block::block_core::BlockManager;
use crate::threading::process::ProcessState;
use crate::threading::thread_control_block::ProcessControlBlock;
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
