use super::{ThreadControlBlock, Tid};
use alloc::boxed::Box;
use core::arch::asm;

pub static mut THREADMANAGER: Option<Box<dyn ThreadManager>> = None;

pub trait ThreadManager {
    fn new() -> Self
    where
        Self: Sized,
        Self: Copy;

    fn allocate_tid(&mut self, thread: Box<ThreadControlBlock>) -> Tid;
    fn deallocate_tid(&mut self) -> ();
}

#[derive(Copy, Clone)]
pub struct ThreadManager128 {
    // list of threads being handled
    pub thread_list: [Option<Box<ThreadControlBlock>>; 128],

    // 4 x 32 = 128 TIDs maximum available
    pid_cache_1: u32,
    pid_cache_2: u32,
    pid_cache_3: u32,
    pid_cache_4: u32,
}

impl ThreadManager for ThreadManager128 {
    fn new() -> ThreadManager128 {
        ThreadManager128 {
            thread_list: [None; 128],
            pid_cache_1: 8589934591,
            pid_cache_2: 8589934591,
            pid_cache_3: 8589934591,
            pid_cache_4: 8589934591,
        }
    }

    fn allocate_tid(&mut self, thread: Box<ThreadControlBlock>) -> Tid {
        // cache1 -> CF if 0, ZF if LSB 1 -> jc -> cache2 ...
        // cache4 -> panic (?) / return -1
        unsafe {
            // asm!(
            //     "tzcnt {c1}, {c1}",
            //     // cache = inout(reg) cache,
            //     // tmp = out(reg) _,
            //     c1 = inout(reg) self.pid_cache_1,
            //     c2 = inout(reg) self.pid_cache_2,
            //     c3 = inout(reg) self.pid_cache_3,
            //     c4 = inout(reg) self.pid_cache_4,
            // );
        }
    }

    fn deallocate_tid(&mut self) -> () {
        
    }
}

pub fn initialize_thread_manager() {
    unsafe {
        THREADMANAGER = Some(Box::new(ThreadManager128::new()));
    }
}
