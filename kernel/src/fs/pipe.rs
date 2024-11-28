use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::fmt::{Debug, Formatter};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::sync::mutex::sleep::SleepMutex;
use crate::sync::semaphore::Semaphore;

pub struct PipeInner {
    pub read_ends: AtomicUsize,
    pub write_ends: AtomicUsize,

    pub semaphore: Semaphore,
    pub contents: SleepMutex<VecDeque<u8>>,
}

pub struct PipeReadEnd(pub Arc<PipeInner>);
pub struct PipeWriteEnd(pub Arc<PipeInner>);

impl PipeInner {
    pub fn new() -> Self {
        Self {
            read_ends: AtomicUsize::new(0),
            write_ends: AtomicUsize::new(0),

            semaphore: Semaphore::new(0),
            contents: SleepMutex::new(VecDeque::new())
        }
    }
}

impl PipeInner {
    pub fn read_end(inner: Arc<PipeInner>) -> PipeReadEnd {
        inner.read_ends.fetch_add(1, Ordering::SeqCst);
        
        PipeReadEnd(inner)
    }
    
    pub fn write_end(inner: Arc<PipeInner>) -> PipeWriteEnd {
        inner.write_ends.fetch_add(1, Ordering::SeqCst);

        PipeWriteEnd(inner)
    }
}

impl Clone for PipeReadEnd {
    fn clone(&self) -> Self {
        self.0.read_ends.fetch_add(1, Ordering::SeqCst);
        
        Self(self.0.clone())
    }
}

impl Clone for PipeWriteEnd {
    fn clone(&self) -> Self {
        self.0.write_ends.fetch_add(1, Ordering::SeqCst);
        
        Self(self.0.clone())
    }
}

impl Drop for PipeReadEnd {
    fn drop(&mut self) {
        self.0.read_ends.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Drop for PipeWriteEnd {
    fn drop(&mut self) {
        self.0.write_ends.fetch_sub(1, Ordering::SeqCst);
    }
}

// Debug Implementations for OpenFile
impl Debug for PipeReadEnd {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Read Pipe End")
    }
}

impl Debug for PipeWriteEnd {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Pipe Write End")
    }
}
