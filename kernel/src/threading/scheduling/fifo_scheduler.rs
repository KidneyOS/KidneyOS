use alloc::{boxed::Box, collections::VecDeque};

use super::super::{ThreadControlBlock, Tid};

use super::scheduler::Scheduler;

pub struct FIFOScheduler {
    ready_queue: VecDeque<Box<ThreadControlBlock>>,
}

// TODO: Will be removed, requires a change to stack type.
// SAFETY: Schedulers should be run with interrupts disabled.
unsafe impl Sync for FIFOScheduler {}

impl Scheduler for FIFOScheduler {
    fn new() -> FIFOScheduler {
        FIFOScheduler {
            ready_queue: VecDeque::new(),
        }
    }

    fn push(&mut self, thread: Box<ThreadControlBlock>) {
        self.ready_queue.push_back(thread);
    }

    fn pop(&mut self) -> Option<Box<ThreadControlBlock>> {
        self.ready_queue.pop_front()
    }

    fn remove(&mut self, _tid: Tid) -> Option<Box<ThreadControlBlock>> {
        let pos = self.ready_queue.iter().position(|tcb| tcb.tid == _tid);
        self.ready_queue.remove(pos?)
    }
}
