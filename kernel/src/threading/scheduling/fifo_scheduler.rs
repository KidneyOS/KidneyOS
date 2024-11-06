use super::super::ThreadControlBlock;
use super::scheduler::Scheduler;
use crate::{
    sync::rwlock::sleep::{RwLock, RwLockWriteGuard},
    threading::process::Tid,
};
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::cell::{RefCell, RefMut};

pub struct FIFOScheduler {
    ready_queue: VecDeque<Arc<RwLock<ThreadControlBlock>>>,
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

    fn push(&mut self, thread: Arc<RwLock<ThreadControlBlock>>) {
        self.ready_queue.push_back(thread);
    }

    fn pop(&mut self) -> Option<Arc<RwLock<ThreadControlBlock>>> {
        self.ready_queue.pop_front()
    }

    fn remove(&mut self, _tid: Tid) -> Option<Arc<RwLock<ThreadControlBlock>>> {
        let pos = self
            .ready_queue
            .iter()
            .position(|tcb| tcb.read().tid == _tid);
        self.ready_queue.remove(pos?)
    }

    fn get_mut(&mut self, _tid: Tid) -> Option<RwLockWriteGuard<'_, ThreadControlBlock>> {
        let pos = self
            .ready_queue
            .iter()
            .position(|tcb| tcb.read().tid == _tid)?;
        let tcb = self.ready_queue.get_mut(pos)?;
        Some(tcb.write())
    }
}
