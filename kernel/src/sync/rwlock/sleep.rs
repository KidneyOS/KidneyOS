use crate::sync::mutex::Mutex;
use crate::system::running_thread_tid;
use crate::threading::process::Tid;
use crate::threading::thread_sleep::{thread_sleep, thread_wakeup};
use alloc::collections::VecDeque;
use core::cell::UnsafeCell;

struct Waiter {
    is_writer: bool,
    tid: Tid,
}

struct RwLockState {
    reader_count: usize,
    any_writer: bool,
    wait_queue: VecDeque<Waiter>,
}

/// A read-write lock, like `std::sync::RwLock`.
///
/// This lock can be acquired for either reading (`&T`) or writing (`&mut T`).
/// It allows any number of concurrent readers, but only one writer at a time.
pub struct RwLock<T> {
    state: Mutex<RwLockState>,
    data: UnsafeCell<T>,
}

pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<T> core::ops::Deref for RwLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: The fact that we have a read guard means that there are no writers currently.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::Deref for RwLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: The fact that we have a write guard means that there are no other readers currently.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The fact that we have a write guard means that there are no other readers or writers currently.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        let mut state = self.lock.state.lock();
        let needs_wakeup = state.reader_count == 1 && !state.wait_queue.is_empty();
        debug_assert!(!state.any_writer);
        state.reader_count -= 1;
        drop(state);
        if needs_wakeup {
            // we were the last reader, so wake up any sleeping writers
            self.lock.wakeup();
        }
    }
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        let mut state = self.lock.state.lock();
        let needs_wakeup = !state.wait_queue.is_empty();
        debug_assert!(state.reader_count == 0);
        state.any_writer = false;
        drop(state);
        if needs_wakeup {
            // wake up any sleeping readers/writers
            self.lock.wakeup();
        }
    }
}

impl<T> RwLock<T> {
    /// Create new RwLock with data
    pub const fn new(data: T) -> Self {
        Self {
            state: Mutex::new(RwLockState {
                any_writer: false,
                reader_count: 0,
                wait_queue: VecDeque::new(),
            }),
            data: UnsafeCell::new(data),
        }
    }
    /// Acquire the lock for reading.
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        loop {
            let mut state = self.state.lock();
            if state.reader_count < usize::MAX && !state.any_writer {
                state.reader_count += 1;
                // SAFETY: there are no writers, since any_writers is false
                return RwLockReadGuard { lock: self };
            }
            state.wait_queue.push_back(Waiter {
                tid: running_thread_tid(),
                is_writer: false,
            });
            drop(state);
            thread_sleep();
        }
    }
    /// Acquire the lock for writing.
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        loop {
            let mut state = self.state.lock();
            if state.reader_count == 0 && !state.any_writer {
                state.any_writer = true;
                // SAFETY: there are no readers or writers, according to state.
                return RwLockWriteGuard { lock: self };
            }
            state.wait_queue.push_back(Waiter {
                tid: running_thread_tid(),
                is_writer: true,
            });
            drop(state);
            thread_sleep();
        }
    }
    /// Wake up sleeping threads waiting on the lock.
    fn wakeup(&self) {
        while let Some(waiter) = self.state.lock().wait_queue.pop_front() {
            thread_wakeup(waiter.tid);
            if waiter.is_writer {
                // no point in waking up more threads since they won't succeed in acquiring the lock
                break;
            }
        }
    }
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> From<T> for RwLock<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        T::default().into()
    }
}
