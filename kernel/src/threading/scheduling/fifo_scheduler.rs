use core::cell::RefCell;
use alloc::rc::Rc;

use alloc::collections::VecDeque;

use super::super::{ThreadControlBlock, Tid};

use super::scheduler::Scheduler;

pub struct FIFOScheduler {
    ready_queue: VecDeque<Rc<RefCell<ThreadControlBlock>>>,
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

    fn push(&mut self, thread: Rc<RefCell<ThreadControlBlock>>) {
        self.ready_queue.push_back(thread);
    }

    fn pop(&mut self) -> Option<Rc<RefCell<ThreadControlBlock>>> {
        self.ready_queue.pop_front()
    }

    fn remove(&mut self, tid: Tid) -> Option<Rc<RefCell<ThreadControlBlock>>> {
        let pos = self.ready_queue.iter().position(|tcb| tcb.borrow().tid == tid);
        self.ready_queue.remove(pos?)
    }
}
