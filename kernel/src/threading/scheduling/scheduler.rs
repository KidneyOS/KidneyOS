use super::super::ThreadControlBlock;
use crate::threading::process::Tid;
use alloc::boxed::Box;

pub trait Scheduler {
    fn new() -> Self
    where
        Self: Sized,
        Self: Sync;

    fn push(&mut self, thread: Box<ThreadControlBlock>);
    fn pop(&mut self) -> Option<Box<ThreadControlBlock>>;
    fn remove(&mut self, tid: Tid) -> Option<Box<ThreadControlBlock>>;
    fn get_mut(&mut self, tid: Tid) -> Option<&mut ThreadControlBlock>;
}
