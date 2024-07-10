use super::super::{ThreadControlBlock, Tid};
use alloc::boxed::Box;

pub trait ThreadManager {
    fn new() -> Self
    where 
        Self: Sized;
    fn add(self: &mut Self, thread:Box<ThreadControlBlock>) -> Tid;
    fn remove(self: &mut Self,  tid: Tid) -> Box<ThreadControlBlock>;
}
