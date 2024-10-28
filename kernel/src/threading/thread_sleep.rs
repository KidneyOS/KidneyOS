use super::{scheduling::scheduler_yield_and_block, thread_control_block::ThreadStatus};
use crate::system::unwrap_system;
use crate::threading::process::Tid;

pub fn thread_sleep() {
    scheduler_yield_and_block();
}

pub fn thread_wakeup(tid: Tid) {
    let threads = &unwrap_system().threads;
    if let Some(tcb) = threads.scheduler.lock().get_mut(tid) {
        tcb.status = ThreadStatus::Ready;
    }
}
