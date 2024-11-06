use core::borrow::BorrowMut;

use crate::block::block_core::BlockManager;
use crate::drivers::input::input_core::InputBuffer;
use crate::sync::mutex::Mutex;
use crate::sync::rwlock::sleep::{RwLockReadGuard, RwLockWriteGuard};
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
/// SYSTEM/thread references cannot be accessed simultaneously on different threads.
pub fn running_thread() -> RwLockReadGuard<'static, ThreadControlBlock> {
    let tcb = unsafe {
        unwrap_system()
            .threads
            .running_thread
            .as_ref()
            .unwrap()
            .read()
    };
    tcb
}

/// Get reference to running process (panicks if no process is running)
///
/// # Safety
///
/// SYSTEM/thread references cannot be accessed simultaneously on different threads.
#[allow(dead_code)]
pub fn running_thread_mut() -> RwLockWriteGuard<'static, ThreadControlBlock> {
    let tcb = unsafe {
        unwrap_system_mut()
            .threads
            .running_thread
            .as_mut()
            .unwrap()
            .borrow_mut()
            .write()
    };
    tcb
}

pub fn running_thread_tid() -> Tid {
    running_thread().tid
}

/// Get reference to running process (panicks if no process is running)
///
/// # Safety
///
/// SYSTEM/process references cannot be accessed simultaneously on different threads.
pub fn running_process() -> RwLockReadGuard<'static, ProcessControlBlock> {
    // running_thread().pcb.read()
    let pid = running_thread().pcb.read().pid;
    unsafe { unwrap_system().process.table.get(pid).unwrap() }
}

/// Get mutable reference to running process (panicks if no process is running)
///
/// # Safety
///
/// SYSTEM/process references cannot be accessed simultaneously on different threads.
pub fn running_process_mut() -> RwLockWriteGuard<'static, ProcessControlBlock> {
    // running_thread_mut().pcb.borrow_mut()
    let pid = running_thread().pcb.read().pid;
    unsafe { unwrap_system_mut().process.table.get_mut(pid).unwrap() }
}

pub fn running_thread_pid() -> Pid {
    running_thread().pcb.read().pid
}

// Returns zero if parent process is 'None' (implying kernel process)
pub fn running_thread_ppid() -> Pid {
    running_process()
        .ppcb
        .as_ref()
        .map(|ppcb| ppcb.read().pid)
        .unwrap_or(0)
}
