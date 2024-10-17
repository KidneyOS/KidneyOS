use crate::interrupts::mutex_irq::MutexIrq;
use crate::system::{unwrap_system, unwrap_system_mut, SYSTEM};
use crate::threading::process::{AtomicTid, Tid};
use crate::threading::thread_sleep::{thread_sleep, thread_wakeup};
use alloc::collections::VecDeque;
use core::sync::atomic::Ordering::{AcqRel, Acquire, Release};
use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
};

pub struct SleepMutex<T: ?Sized> {
    holding_thread: AtomicTid,
    wait_queue: MutexIrq<VecDeque<Tid>>,
    data: UnsafeCell<T>,
}

pub struct SleepMutexGuard<'a, T: ?Sized + 'a> {
    mutex: Option<&'a SleepMutex<T>>,
}

// Same unsafe impls as `std::sync::Mutex`
unsafe impl<T: ?Sized + Send> Sync for SleepMutex<T> {}
unsafe impl<T: ?Sized + Send> Send for SleepMutex<T> {}

unsafe impl<T: ?Sized + Sync> Sync for SleepMutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Send> Send for SleepMutexGuard<'_, T> {}

impl<'a, T> SleepMutexGuard<'a, T> {
    pub fn unlock(&mut self) {
        if let Some(mutex) = self.mutex.take() {
            mutex.unlock();
        }
    }
}

// Ensure mutex is released if dropped (such as in the event of a panic)
impl<'a, T: ?Sized> Drop for SleepMutexGuard<'a, T> {
    fn drop(&mut self) {
        if let Some(mutex) = self.mutex.take() {
            mutex.unlock();
        }
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for SleepMutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(
            &self
                .mutex
                .as_ref()
                .expect("No inner mutex present")
                .data
                .get(),
            f,
        )
    }
}

impl<'a, T: ?Sized> Deref for SleepMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.as_ref().unwrap().data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for SleepMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.as_mut().unwrap().data.get() }
    }
}

impl<T: ?Sized + Default> Default for SleepMutex<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> From<T> for SleepMutex<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<T> SleepMutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            holding_thread: AtomicTid::new(0),
            wait_queue: MutexIrq::new(VecDeque::new()),
            data: UnsafeCell::new(data),
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: ?Sized> SleepMutex<T> {
    #[must_use = "Mutex is released when guard falls out of scope."]
    pub fn lock(&self) -> SleepMutexGuard<T> {
        let current_tid = unsafe {
            unwrap_system_mut()
                .threads
                .running_thread
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };

        loop {
            // If no thread is holding the mutex, grab it.
            let _ = self
                .holding_thread
                .compare_exchange(0, current_tid, AcqRel, Acquire);
            // If we are the owner of the mutex, break.
            // Note that holding_thread can be set to current_tid either by the line
            // above, or by unlock().
            if self.holding_thread.load(Acquire) == current_tid {
                break;
            }

            let mut wait_queue = self.wait_queue.lock();
            if !wait_queue.contains(&current_tid) {
                wait_queue.push_back(current_tid);
            }
            drop(wait_queue);
            thread_sleep();
        }

        SleepMutexGuard { mutex: Some(self) }
    }

    fn unlock(&self) {
        let running_tid = unsafe {
            unwrap_system()
                .threads
                .running_thread
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };

        if self.holding_thread.load(Acquire) != running_tid {
            return;
        }

        let next = self.wait_queue.lock().pop_front();
        match next {
            None => {
                self.holding_thread.store(0, Release);
            }
            Some(next_thread) => {
                self.holding_thread.store(next_thread, Release);
                thread_wakeup(next_thread);
            }
        }
    }

    pub fn is_locked(&self) -> bool {
        self.holding_thread.load(Acquire) != 0
    }

    pub fn try_lock(&self) -> bool {
        let current_tid = unsafe {
            unwrap_system()
                .threads
                .running_thread
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };
        self.holding_thread
            .compare_exchange(0, current_tid, AcqRel, Acquire)
            .is_ok()
    }

    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: no other references can exist, since we have a mut reference to self.
        unsafe { &mut *self.data.get() }
    }
}
