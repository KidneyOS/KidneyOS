use alloc::collections::VecDeque;

use super::super::Tid;

use super::scheduler::Scheduler;

pub struct FIFOScheduler {
    ready_queue: VecDeque<Tid>,
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

    fn push(&mut self, tid: Tid) {
        self.ready_queue.push_back(tid);
    }

    fn pop(&mut self) -> Option<Tid> {
        self.ready_queue.pop_front()
    }

    fn remove(&mut self, _tid: Tid) -> bool {
        false
    }
}
