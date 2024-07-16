use alloc::{boxed::Box, collections::VecDeque};

use super::super::{ThreadControlBlock, Tid};

use super::scheduler::Scheduler;

pub struct FIFOScheduler<'a> {
    ready_queue: VecDeque<&'a Box<ThreadControlBlock>>,
}

// TODO: Will be removed, requires a change to stack type.
// SAFETY: Schedulers should be run with interrupts disabled.
unsafe impl Sync for FIFOScheduler<'_> {}

impl<'a> Scheduler<'a> for FIFOScheduler<'a> {
    fn new() -> FIFOScheduler<'a> {
        FIFOScheduler {
            ready_queue: VecDeque::new(),
        }
    }

    fn push(&mut self, thread: &'a Box<ThreadControlBlock>) {
        self.ready_queue.push_back(thread);
    }

    fn pop(&mut self) -> Option<&Box<ThreadControlBlock>> {
        self.ready_queue.pop_front()
    }

    fn remove(&mut self, _tid: Tid) -> bool {
        false
    }
}
