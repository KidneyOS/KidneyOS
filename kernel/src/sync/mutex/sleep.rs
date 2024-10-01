use crate::{
    interrupts::{intr_disable, intr_enable},
    threading::{
        thread_control_block::Tid,
        thread_sleep::{thread_sleep, thread_wakeup},
        RUNNING_THREAD,
    },
};
use alloc::collections::VecDeque;

pub struct SleepMutex {
    aquired: bool,
    holding_thread: Option<Tid>,
    wait_queue: VecDeque<Tid>,
}

pub struct SleepMutexGuard<'a> {
    mutex: Option<&'a mut SleepMutex>,
}

impl<'a> SleepMutexGuard<'a> {
    pub fn unlock(&mut self) {
        intr_disable();
        if let Some(mutex) = self.mutex.take() {
            mutex.unlock();
        }
        intr_enable();
    }

    pub fn is_locked(&self) -> bool {
        self.mutex.is_some()
    }
}

// Ensure mutex is released if dropped (such as in the event of a panic)
impl<'a> Drop for SleepMutexGuard<'a> {
    fn drop(&mut self) {
        if let Some(mutex) = self.mutex.take() {
            mutex.unlock();
        }
    }
}

impl SleepMutex {
    pub const fn new() -> Self {
        Self {
            aquired: false,
            holding_thread: None,
            wait_queue: VecDeque::new(),
        }
    }

    pub fn lock(&mut self) -> SleepMutexGuard {
        intr_disable();

        let current_tid = unsafe {
            RUNNING_THREAD
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };

        while self.aquired {
            if !self.wait_queue.contains(&current_tid) {
                self.wait_queue.push_back(current_tid);
            }
            thread_sleep();
        }

        self.aquired = true;
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
            intr_enable();
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
            self.aquired = false;
            self.holding_thread = None;
        }
    }

    pub unsafe fn force_unlock(&mut self) {
        intr_disable();

        if let Some(next_thread) = self.wait_queue.pop_front() {
            thread_wakeup(next_thread);
        }

        self.aquired = false;
        self.holding_thread = None;
        intr_enable();
    }

    pub fn is_locked(&mut self) -> bool {
        self.aquired
    }

    pub fn try_lock(&mut self) -> bool {
        intr_disable();

        if self.aquired {
            intr_enable();
            return false;
        }

        let current_tid = unsafe {RUNNING_THREAD
            .as_ref()
            .expect("why is nothing running?")
            .tid };

        self.aquired = true;
        self.holding_thread = Some(current_tid);
        intr_enable();
        true
    }
}

// Release all threads when mutex is dropped
impl Drop for SleepMutex {
    fn drop(&mut self) {
        intr_disable();

        while let Some(tid) = self.wait_queue.pop_front() {
            thread_wakeup(tid);
        }

        self.aquired = false;
        self.holding_thread = None;
        intr_enable();
    }
}
