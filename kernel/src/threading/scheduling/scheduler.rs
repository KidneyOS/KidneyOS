use super::super::{ThreadControlBlock, Tid};
use crate::alloc::boxed::Box;

pub trait Scheduler<'a> {
    fn new() -> Self
    where
        Self: Sized,
        Self: Sync;

    fn push(&mut self, thread: &'a Box<ThreadControlBlock>);
    fn pop(&mut self) -> Option<&Box<ThreadControlBlock>>;
    fn remove(&mut self, tid: Tid) -> bool;
}
