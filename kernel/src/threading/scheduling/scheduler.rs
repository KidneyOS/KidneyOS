use super::super::Tid;

pub trait Scheduler {
    fn new() -> Self
    where
        Self: Sized,
        Self: Sync;

    fn push(&mut self, tid: Tid);
    fn pop(&mut self) -> Option<Tid>;
    fn remove(&mut self, tid: Tid) -> bool;
}
