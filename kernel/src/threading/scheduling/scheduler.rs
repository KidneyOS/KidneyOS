use super::super::ThreadControlBlock;
use crate::threading::process::Tid;
use alloc::rc::Rc;
use core::cell::RefCell;

pub trait Scheduler {
    fn new() -> Self
    where
        Self: Sized,
        Self: Sync;

    fn push(&mut self, thread: Rc<RefCell<ThreadControlBlock>>);
    fn pop(&mut self) -> Option<Rc<RefCell<ThreadControlBlock>>>;
    fn remove(&mut self, tid: Tid) -> Option<Rc<RefCell<ThreadControlBlock>>>;
    fn get_mut(&mut self, tid: Tid) -> Option<&mut ThreadControlBlock>;
}
