use super::ThreadControlBlock;
use super::Tid;
use alloc::collections::VecDeque;

trait Scheduler {
    fn new() -> Self
    where
        Self: Sized;

    fn push(&mut self, thread: ThreadControlBlock);
    fn pop(&mut self) -> Option<ThreadControlBlock>;
    fn remove(&mut self, tid: Tid) -> bool;
}

struct FIFOScheduler {
    ready_queue: VecDeque<ThreadControlBlock>,
}

impl Scheduler for FIFOScheduler {
    fn new() -> FIFOScheduler {
        FIFOScheduler {
            ready_queue: VecDeque::new(),
        }
    }

    fn push(&mut self, thread: ThreadControlBlock) {
        self.ready_queue.push_back(thread);
    }

    fn pop(&mut self) -> Option<ThreadControlBlock> {
        self.ready_queue.pop_front()
    }

    fn remove(&mut self, _tid: Tid) -> bool {
        false
    }
}
