use super::super::{ThreadControlBlock, Tid};

pub trait ThreadManager {
    fn new() -> Self
    where 
        Self: Sized;
    fn add(&mut self, thread: ThreadControlBlock) -> Tid;
    fn remove(&mut self,  tid: Tid) -> ThreadControlBlock;
    fn get(&mut self, tid: Tid) -> ThreadControlBlock;
    fn set(&mut self, thread: ThreadControlBlock) -> Tid;
}
