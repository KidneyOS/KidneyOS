use crate::block::block_core::BlockManager;
use crate::sync::mutex::Mutex;
use crate::sync::rwlock::sleep::RwLock;
use crate::threading::process::{Pid, ProcessState, Tid};
use crate::threading::thread_control_block::ProcessControlBlock;
use crate::threading::ThreadState;
use alloc::sync::Arc;
use once_cell::race::OnceBox;

pub struct SystemState {
    pub threads: ThreadState,
    pub process: ProcessState,

    pub block_manager: RwLock<BlockManager>,
}

impl core::fmt::Debug for SystemState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // probably isn't very helpful to show all the fields here.
        write!(f, "<SystemState>")
    }
}

/*
    We have to put SystemState in a Box because OnceCell can only be used with
    std or critical-section (which doesn't seem to work).
    (There's no spinlock version of OnceCell, because of fears about
    priority inversion, which is irrelevant for us.)
*/
static SYSTEM: OnceBox<SystemState> = OnceBox::new();

pub fn init_system(state: SystemState) {
    SYSTEM
        .set(alloc::boxed::Box::new(state))
        .expect("System initialized twice");
}

pub fn unwrap_system() -> &'static SystemState {
    SYSTEM.get().expect("System not initialized.")
}

/// Get reference to running process (panicks if no process is running)
pub fn running_process() -> Arc<Mutex<ProcessControlBlock>> {
    let system = unwrap_system();
    let pid = system.threads.running_thread.lock().as_ref().unwrap().pid;
    system.process.table.get(pid).unwrap()
}

pub fn running_thread_pid() -> Pid {
    let tcb = unwrap_system().threads.running_thread.lock();
    tcb.as_ref().expect("Why is nothing running?").as_ref().pid
}

pub fn running_thread_ppid() -> Pid {
    running_process().lock().ppid
}

pub fn running_thread_tid() -> Tid {
    unwrap_system()
        .threads
        .running_thread
        .lock()
        .as_ref()
        .expect("no running thread")
        .tid
}
