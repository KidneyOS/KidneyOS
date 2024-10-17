use super::thread_control_block::{Pid, ProcessControlBlock};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;

pub static mut PROCESS_TABLE: Option<Box<ProcessTable>> = None;

pub fn initialize_process_table() {
    unsafe {
        PROCESS_TABLE = Some(Box::new(ProcessTable::new()));
    }
}

pub struct ProcessTable {
    table: BTreeMap<Pid, Box<ProcessControlBlock>>,
}

impl ProcessTable {
    pub fn new() -> ProcessTable {
        ProcessTable {
            table: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, pcb: Box<ProcessControlBlock>) {
        assert!(
            !self.table.contains_key(&pcb.pid),
            "PCB with pid {} already added to process table.",
            pcb.pid
        );
        self.table.insert(pcb.pid, pcb);
    }

    #[allow(unused)]
    pub fn remove(&mut self, pid: Pid) -> Option<Box<ProcessControlBlock>> {
        self.table.remove(&pid)
    }

    #[allow(unused)]
    pub fn get(&self, pid: Pid) -> Option<&ProcessControlBlock> {
        self.table.get(&pid).map(|pcb| &**pcb)
    }

    #[allow(unused)]
    pub fn get_mut(&mut self, pid: Pid) -> Option<&mut ProcessControlBlock> {
        self.table.get_mut(&pid).map(|pcb| &mut **pcb)
    }
}
