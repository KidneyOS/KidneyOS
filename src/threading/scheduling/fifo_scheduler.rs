use alloc::collections::VecDeque;

use super::super::{TID, ThreadControlBlock};

use super::scheduler::Scheduler;

pub struct FIFOScheduler {

    ready_queue: VecDeque<ThreadControlBlock>

}

unsafe impl Sync for FIFOScheduler {}

impl Scheduler for FIFOScheduler {

    fn new() -> FIFOScheduler {
        return FIFOScheduler {
            ready_queue: VecDeque::new()
        };
    }

    fn push(&mut self, thread: ThreadControlBlock) -> () {

        self.ready_queue.push_back(thread);

    }

    fn pop(&mut self) -> Option<ThreadControlBlock> {

        return self.ready_queue.pop_front();

    }

    fn remove(&mut self, tid: TID) -> bool {
        return false;
    }

}
