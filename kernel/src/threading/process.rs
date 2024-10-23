use super::thread_control_block::ProcessControlBlock;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU16, Ordering};

pub type Pid = u16;
pub type Tid = u16;
pub type AtomicPid = AtomicU16;
pub type AtomicTid = AtomicU16;

#[derive(Default)]
pub struct ProcessTable {
    content: BTreeMap<Pid, Box<ProcessControlBlock>>,
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
    pub fn add(&mut self, pcb: Box<ProcessControlBlock>) {
        assert!(
            !self.content.contains_key(&pcb.pid),
            "PCB with pid {} already added to process table.",
            pcb.pid
        );
        self.content.insert(pcb.pid, pcb);
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, pid: Pid) -> Option<Box<ProcessControlBlock>> {
        self.content.remove(&pid)
    }

    #[allow(dead_code)]
    pub fn get(&self, pid: Pid) -> Option<&ProcessControlBlock> {
        self.content.get(&pid).map(|pcb| &**pcb)
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self, pid: Pid) -> Option<&mut ProcessControlBlock> {
        self.content.get_mut(&pid).map(|pcb| &mut **pcb)
    }
}
