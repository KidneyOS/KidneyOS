use super::super::{ThreadControlBlock, Tid};
use alloc::boxed::Box;

pub trait ThreadManager {
    fn new() -> Self
    where 
        Self: Sized;
    fn add(&mut self, thread: Box<ThreadControlBlock>) -> Tid;
    fn remove(&mut self,  tid: Tid) -> Box<ThreadControlBlock>;
    fn get(&mut self, tid: Tid) -> Box<ThreadControlBlock>;
    fn set(&mut self, thread: Box<ThreadControlBlock>) -> Tid;
}
