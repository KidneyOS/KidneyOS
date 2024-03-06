use super::super::{TID, ThreadControlBlock};

pub trait Scheduler {

    fn new() -> Self
    where
        Self: Sized,
        Self: Sync;

    fn push(&mut self, thread: ThreadControlBlock) -> ();
    fn pop(&mut self) -> Option<ThreadControlBlock>;
    fn remove(&mut self, tid: TID) -> bool;

}

pub struct NullScheduler {}

impl Scheduler for NullScheduler {
    fn new() -> NullScheduler { NullScheduler {  } }

    fn push(&mut self, thread: ThreadControlBlock) -> () { }
    fn pop(&mut self) -> Option<ThreadControlBlock> { None }
    fn remove(&mut self, tid: TID) -> bool { false }
}
