use super::super::ThreadControlBlock;
use super::scheduler::Scheduler;
use crate::threading::process::Tid;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use core::cell::RefCell;

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

    fn remove(&mut self, _tid: Tid) -> Option<Rc<RefCell<ThreadControlBlock>>> {
        let pos = self
            .ready_queue
            .iter()
            .position(|tcb| tcb.borrow().tid == _tid);
        self.ready_queue.remove(pos?)
    }

    fn get_mut(&mut self, _tid: Tid) -> Option<&mut ThreadControlBlock> {
        let pos = self
            .ready_queue
            .iter()
            .position(|tcb| tcb.borrow().tid == _tid)?;
        Some(self.ready_queue.get_mut(pos)?.get_mut())
    }
}
