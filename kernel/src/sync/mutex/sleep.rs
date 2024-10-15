use crate::{
    interrupts::{intr_disable, intr_enable},
    threading::{
        thread_control_block::Tid,
        thread_sleep::{thread_sleep, thread_wakeup},
        RUNNING_THREAD,
    },
};
use alloc::collections::VecDeque;
use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
};

pub struct SleepMutex<T: ?Sized> {
    holding_thread: Option<Tid>,
    wait_queue: VecDeque<Tid>,
    data: UnsafeCell<T>,
}

pub struct SleepMutexGuard<'a, T: ?Sized + 'a> {
    mutex: Option<&'a mut SleepMutex<T>>,
}

// Same unsafe impls as `std::sync::Mutex`
unsafe impl<T: ?Sized + Send> Sync for SleepMutex<T> {}
unsafe impl<T: ?Sized + Send> Send for SleepMutex<T> {}

unsafe impl<T: ?Sized + Sync> Sync for SleepMutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Send> Send for SleepMutexGuard<'_, T> {}

impl<'a, T> SleepMutexGuard<'a, T> {
    pub fn unlock(&mut self) {
        intr_disable();
        if let Some(mutex) = self.mutex.take() {
            mutex.unlock();
        }
        intr_enable();
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
            holding_thread: None,
            wait_queue: VecDeque::new(),
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
    #[must_use = "Mutex is released when guard falls out of scpe"]
    pub fn lock(&mut self) -> SleepMutexGuard<T> {
        intr_disable();

        let current_tid = unsafe {
            RUNNING_THREAD
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };

        while self.is_locked() {
            if !self.wait_queue.contains(&current_tid) {
                self.wait_queue.push_back(current_tid);
            }
            thread_sleep();
        }

        self.holding_thread = Some(current_tid);
        intr_enable();

        SleepMutexGuard { mutex: Some(self) }
    }

    pub fn unlock(&mut self) {
        let running_tid = unsafe {
            RUNNING_THREAD
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };

        if self.holding_thread != Some(running_tid) {
            return;
        }

        if !self.wait_queue.is_empty() {
            let next_thread = self
                .wait_queue
                .pop_front()
                .expect("No item in wait queue despite non-zero size.");
            self.holding_thread = Some(next_thread);
            thread_wakeup(next_thread);
        } else {
            self.holding_thread = None;
        }
    }

    pub unsafe fn force_unlock(&mut self) {
        intr_disable();

        if let Some(next_thread) = self.wait_queue.pop_front() {
            thread_wakeup(next_thread);
        }

        self.holding_thread = None;
        intr_enable();
    }

    pub fn is_locked(&self) -> bool {
        self.holding_thread.is_some()
    }

    pub fn try_lock(&mut self) -> bool {
        intr_disable();

        if self.is_locked() {
            intr_enable();
            return false;
        }

        let current_tid = unsafe {
            RUNNING_THREAD
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };

        self.holding_thread = Some(current_tid);
        intr_enable();
        true
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}
