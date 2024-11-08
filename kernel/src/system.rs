use crate::block::block_core::BlockManager;
use crate::drivers::input::input_core::InputBuffer;
use crate::fs::fs_manager::RootFileSystem;
use crate::sync::mutex::Mutex;
use crate::sync::rwlock::sleep::RwLock;
use crate::threading::process::{Pid, ProcessState, Tid};
use crate::threading::thread_control_block::ProcessControlBlock;
use crate::threading::ThreadState;
use alloc::sync::Arc;

pub struct SystemState {
    pub threads: ThreadState,
    pub process: ProcessState,

    pub block_manager: RwLock<BlockManager>,
    pub root_filesystem: Mutex<RootFileSystem>,
    pub input_buffer: Mutex<InputBuffer>,
}

impl core::fmt::Debug for SystemState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // probably isn't very helpful to show all the fields here.
        write!(f, "<SystemState>")
    }
}

static mut SYSTEM: core::mem::MaybeUninit<SystemState> = core::mem::MaybeUninit::uninit();
const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;
static SYSTEM_STATE: core::sync::atomic::AtomicU8 =
    core::sync::atomic::AtomicU8::new(UNINITIALIZED);

pub fn init_system(state: SystemState) {
    SYSTEM_STATE
        .compare_exchange(
            UNINITIALIZED,
            INITIALIZING,
            core::sync::atomic::Ordering::Relaxed,
            core::sync::atomic::Ordering::Relaxed,
        )
        .expect("System initialized twice");
    // SAFETY:
    //   - only one thread can successfully exchange UNINITIALIZED with INITIALIZING
    //   - no threads can have references to the system since unwrap_system()
    //     requires SYSTEM_STATE to be set to INITIALIZED
    unsafe { SYSTEM.write(state) };
    SYSTEM_STATE.store(INITIALIZED, core::sync::atomic::Ordering::Release);
}

pub fn unwrap_system() -> &'static SystemState {
    if SYSTEM_STATE.load(core::sync::atomic::Ordering::Acquire) == INITIALIZED {
        // SAFETY: since SYSTEM_STATE = INITIALIZED, the SYSTEM has been initialized.
        unsafe { SYSTEM.assume_init_ref() }
    } else {
        panic!("System not initialized.");
    }
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

pub fn root_filesystem() -> &'static Mutex<RootFileSystem> {
    &unwrap_system().root_filesystem
}
