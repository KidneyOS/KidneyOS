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

impl SleepMutex {
    pub const fn new() -> Self {
        Self {
            aquired: false,
            holding_thread: None,
            wait_queue: VecDeque::<Tid>::new(),
        }
    }

    pub fn lock(&mut self) {
        intr_disable();

        // Add check if thread is already waiting on lock
        let current_tid = unsafe {
            RUNNING_THREAD
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };

        if self.aquired {
            self.wait_queue.push_back(current_tid);
            thread_sleep();
        }

        self.aquired = true;
        self.holding_thread = Some(current_tid);
        intr_enable();
    }

    pub fn unlock(&mut self) {
        intr_disable();

        let running_tid = unsafe {
            RUNNING_THREAD
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };

        if self.holding_thread != Some(running_tid) {
            return;
        }

        if self.wait_queue.len() > 0 {
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
        intr_enable();
    }

    pub unsafe fn force_unlock(&mut self) {
        intr_disable();

        let running_tid = unsafe {
            RUNNING_THREAD
                .as_ref()
                .expect("why is nothing running?")
                .tid
        };
        thread_wakeup(running_tid);

        if let Some(pos) = self.wait_queue.iter().position(|tid| *tid == running_tid) {
            self.wait_queue.remove(pos);
        }

        intr_enable();
    }
}

impl Drop for SleepMutex {
    fn drop(&mut self) {
        // Unsure whether we should panic at this, or just release all waiting threads
        assert!(
            self.wait_queue.is_empty(),
            "Mutex was dropped with thread still waiting"
        );
    }
}
