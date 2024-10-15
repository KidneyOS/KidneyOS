use alloc::rc::Rc;
use core::cell::RefCell;
use super::super::{ThreadControlBlock, Tid};

pub trait Scheduler {
    fn new() -> Self
    where
        Self: Sized,
        Self: Sync;

    fn push(&mut self, thread: Rc<RefCell<ThreadControlBlock>>);
    fn pop(&mut self) -> Option<Rc<RefCell<ThreadControlBlock>>>;
    fn remove(&mut self, tid: Tid) -> Option<Rc<RefCell<ThreadControlBlock>>>;
}
