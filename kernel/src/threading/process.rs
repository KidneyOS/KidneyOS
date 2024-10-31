use super::thread_control_block::ProcessControlBlock;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use core::{
    cell::{Ref, RefCell, RefMut},
    sync::atomic::{AtomicU16, Ordering},
};

pub type Pid = u16;
pub type Tid = u16;
pub type AtomicPid = AtomicU16;
pub type AtomicTid = AtomicU16;

#[derive(Default)]
pub struct ProcessTable {
    content: BTreeMap<Pid, Rc<RefCell<ProcessControlBlock>>>,
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
            panic!("TID overflow"); // TODO: handle overflow properly
        }
        tid
    }
}

impl ProcessTable {
    pub fn add(&mut self, pcb: Rc<RefCell<ProcessControlBlock>>) {
        let pid = pcb.borrow().pid;
        assert!(
            !self.content.contains_key(&pid),
            "PCB with pid {} already added to process table.",
            pcb.borrow().pid
        );
        self.content.insert(pid, pcb);
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, pid: Pid) -> Option<Rc<RefCell<ProcessControlBlock>>> {
        self.content.remove(&pid)
    }

    pub fn get(&self, pid: Pid) -> Option<Ref<'_, ProcessControlBlock>> {
        self.content
            .get(&pid)
            .and_then(|entry| Some(entry.borrow()))
    }

    pub fn get_mut(&mut self, pid: Pid) -> Option<RefMut<'_, ProcessControlBlock>> {
        self.content
            .get_mut(&pid)
            .and_then(|entry| Some(entry.borrow_mut()))
    }
}
