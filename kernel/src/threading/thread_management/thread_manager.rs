use super::super::{ThreadControlBlock, Tid};
use alloc::boxed::Box;

// CONSTRAINT: First allocated TID: 0, Second allocated TID: 1; Always from state
// of no allocated TIDs.

pub trait ThreadManager {
    fn new() -> Self
    where
        Self: Sized;
    fn add(&mut self, thread: Box<ThreadControlBlock>) -> Tid;
    fn remove(&mut self, tid: Tid) -> Box<ThreadControlBlock>;
    fn get(&mut self, tid: Tid) -> Box<ThreadControlBlock>;
    fn set(&mut self, thread: Box<ThreadControlBlock>) -> Tid;
}
