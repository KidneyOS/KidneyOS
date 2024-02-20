use alloc::collections::VecDeque;
use super::ThreadControlBlock;
use super::TID;

trait Scheduler {

    fn new() -> Self
    where
        Self: Sized;

    fn push(&mut self, thread: ThreadControlBlock) -> ();
    fn pop(&mut self) -> Option<ThreadControlBlock>;
    fn remove(&mut self, tid: TID) -> bool;

}

struct FIFOScheduler {

    ready_queue: VecDeque<ThreadControlBlock>

}

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
