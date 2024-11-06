use super::super::ThreadControlBlock;
use crate::{
    sync::rwlock::sleep::{RwLock, RwLockWriteGuard},
    threading::process::Tid,
};
use alloc::sync::Arc;

pub trait Scheduler {
    fn new() -> Self
    where
        Self: Sized,
        Self: Sync;

    fn push(&mut self, thread: Arc<RwLock<ThreadControlBlock>>);
    fn pop(&mut self) -> Option<Arc<RwLock<ThreadControlBlock>>>;
    fn remove(&mut self, tid: Tid) -> Option<Arc<RwLock<ThreadControlBlock>>>;
    fn get_mut(&mut self, tid: Tid) -> Option<RwLockWriteGuard<'_, ThreadControlBlock>>;
}
