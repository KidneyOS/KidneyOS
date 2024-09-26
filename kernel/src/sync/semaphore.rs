#![allow(unused)]

use crate::sync::mutex::TicketMutex;
use alloc::collections::VecDeque;

pub struct Semaphore {
    value: TicketMutex<i32>,
    queue: TicketMutex<VecDeque<()>>,
}

impl Semaphore {
    pub const fn new(count: i32) -> Self {
        Self {
            value: TicketMutex::new(count),
            queue: TicketMutex::new(VecDeque::new()),
        }
    }

    pub fn down(&self) {
        // TODO: Implement semaphore down
    }

    pub fn try_down(&self) -> bool {
        // TODO: Implement semaphore try down
        false
    }

    pub fn up(&self) {
        // TODO: Implement semaphore up
    }
}
