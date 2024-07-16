use super::super::{ThreadControlBlock, Tid};
use alloc::boxed::Box;

pub trait ThreadManager {
    fn new() -> Self
    where 
        Self: Sized;
    fn add(&mut self, thread:Box<ThreadControlBlock>) -> &Box<ThreadControlBlock>;
    fn remove(&mut self,  tid: Tid) -> Box<ThreadControlBlock>;
    fn add_existing(&mut self, thread:Box<ThreadControlBlock>) -> &Box<ThreadControlBlock>;
    unsafe fn get_clone_ptr(&mut self, tid: Tid) -> *mut ThreadControlBlock;
}
