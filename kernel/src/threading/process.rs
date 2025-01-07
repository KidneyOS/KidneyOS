use super::thread_control_block::ProcessControlBlock;
use crate::sync::{mutex::Mutex, rwlock::sleep::RwLock};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU16, Ordering};

pub type Pid = u16;
pub type Tid = u16;
pub type AtomicPid = AtomicU16;
pub type AtomicTid = AtomicU16;

#[derive(Default)]
pub struct ProcessTable {
    content: RwLock<BTreeMap<Pid, Arc<Mutex<ProcessControlBlock>>>>,
}

pub struct ProcessState {
    pub table: ProcessTable,
    next_tid: AtomicTid,
    next_pid: AtomicPid,
}

pub fn create_process_state() -> ProcessState {
    ProcessState {
        table: Default::default(),
        next_tid: AtomicTid::new(1),
        next_pid: AtomicPid::new(1),
    }
}

impl ProcessState {
    pub fn allocate_pid(&self) -> Pid {
        // SAFETY: Atomically accesses a shared variable.
        let pid = self.next_pid.fetch_add(1, Ordering::SeqCst);
        if pid == 0 {
            panic!("PID overflow"); // TODO: handle overflow properly
        }
        pid
    }
    pub fn allocate_tid(&self) -> Tid {
        // SAFETY: Atomically accesses a shared variable.
        let tid = self.next_tid.fetch_add(1, Ordering::SeqCst);
        if tid == 0 {
            panic!("PID overflow"); // TODO: handle overflow properly
        }
        tid
    }
}

impl ProcessTable {
    pub fn add(&self, pcb: ProcessControlBlock) -> Arc<Mutex<ProcessControlBlock>> {
        let pid = pcb.pid;
        let mut content = self.content.write();
        assert!(
            !content.contains_key(&pid),
            "PCB with pid {} already added to process table.",
            pid
        );
        let pcb = Arc::new(Mutex::new(pcb));
        content.insert(pid, pcb.clone());
        pcb
    }

    #[allow(dead_code)]
    pub fn remove(&self, pid: Pid) -> Option<Arc<Mutex<ProcessControlBlock>>> {
        self.content.write().remove(&pid)
    }

    pub fn get(&self, pid: Pid) -> Option<Arc<Mutex<ProcessControlBlock>>> {
        self.content.read().get(&pid).cloned()
    }
}
