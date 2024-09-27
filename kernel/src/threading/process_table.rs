use super::thread_control_block::{Pid, ProcessControlBlock};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;

pub struct ProcessTable {
    table: BTreeMap<Pid, Box<ProcessControlBlock>>,
}

impl ProcessTable {
    #![allow(dead_code)]
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

    pub fn remove(&mut self, pid: Pid) -> Option<Box<ProcessControlBlock>> {
        #[allow(dead_code)]
        self.table.remove(&pid)
    }

    pub fn get(&self, pid: Pid) -> Option<&ProcessControlBlock> {
        #[allow(dead_code)]
        self.table.get(&pid).map(|pcb| &**pcb)
    }
}
