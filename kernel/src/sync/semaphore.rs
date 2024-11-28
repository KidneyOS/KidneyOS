#![allow(unused)]

use crate::sync::mutex::TicketMutex;
use crate::system::unwrap_system;
use crate::threading::process::Tid;
use crate::threading::thread_control_block::ThreadStatus;
use crate::threading::thread_sleep::{thread_sleep, thread_wakeup};
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use crate::interrupts::IntrLevel;
use crate::interrupts::mutex_irq::hold_interrupts;

pub struct SemaphorePermit {
    forgotten: bool,
    inner: Arc<TicketMutex<SemaphoreInner>>,
}

struct SemaphoreInner {
    value: i32,
    queue: VecDeque<Tid>,
}

// Sleep Semaphore
pub struct Semaphore {
    inner: Arc<TicketMutex<SemaphoreInner>>,
}

impl SemaphorePermit {
    fn new(inner: Arc<TicketMutex<SemaphoreInner>>) -> Self {
        Self {
            forgotten: false,
            inner,
        }
    }

    pub fn forget(mut self) {
        self.forgotten = true;

        drop(self)
    }
}

impl SemaphoreInner {
    fn post(&mut self) {
        self.value += 1;

        // Wake one thread.
        if let Some(tid) = self.queue.pop_front() {
            thread_wakeup(tid)
        }
    }
}

impl Semaphore {
    pub fn new(value: i32) -> Self {
        Self {
            inner: Arc::new(TicketMutex::new(SemaphoreInner {
                value,
                queue: VecDeque::new(),
            })),
        }
    }

    pub fn post(&self) {
        self.inner.lock().post()
    }

    #[must_use]
    pub fn acquire(&self) -> SemaphorePermit {
        loop {
            {
                // Release inner at the end of this scope, so we don't hold it through thread_sleep.
                let mut inner = self.inner.lock();

                if inner.value > 0 {
                    inner.value -= 1;

                    return SemaphorePermit::new(self.inner.clone());
                }

                let running_tid = unsafe {
                    unwrap_system()
                        .threads
                        .running_thread
                        .lock()
                        .as_ref()
                        .expect("why is nothing running?")
                        .tid
                };

                // Push ourselves (back) on the wait queue.
                if !inner.queue.contains(&running_tid) {
                    inner.queue.push_back(running_tid);
                }
            }

            let _guard = hold_interrupts(IntrLevel::IntrOn);
            
            thread_sleep();
        }
    }

    #[must_use]
    pub fn try_acquire(&self) -> Option<SemaphorePermit> {
        let mut inner = self.inner.lock();

        if inner.value > 0 {
            inner.value -= 1;

            Some(SemaphorePermit::new(self.inner.clone()))
        } else {
            None
        }
    }
}

impl Drop for SemaphorePermit {
    fn drop(&mut self) {
        if !self.forgotten {
            self.inner.lock().post()
        }
    }
}
