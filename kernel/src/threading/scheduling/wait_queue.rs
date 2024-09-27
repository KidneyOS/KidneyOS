pub struct WaitQueue {
    wait_queue: HashSet<Tid, Box<ThreadControlBlock>>,
}

impl WaitQueue {
    pub fn new() -> WaitQueue {
        WaitQueue {
            wait_queue: HashSet::new()
        }
    }

    pub fn add(&mut self, tid: Tid, tcb: Box<ThreadControlBlock>) {
        assert!(!self.wait_queue.contains_key(tid), "TCB with tid: {} already in wait queue", tid);
        self.wait_queue.add(tid, tcb);
    }

    pub fn remove(&mut self, tid: Tid) -> Option<Box<ThreadControlBlock>> {
        self.wait_queue.remove(tid)
    }
}