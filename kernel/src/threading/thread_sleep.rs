use super::{scheduling::scheduler_yield_and_block, thread_control_block::ThreadStatus};
use crate::system::SYSTEM;
use crate::threading::process::Tid;

pub fn thread_sleep() {
    scheduler_yield_and_block();
}

pub fn thread_wakeup(tid: Tid) {
    let threads = unsafe { &mut SYSTEM.as_mut().expect("System not initialized.").threads };
    if let Some(tcb) = threads.scheduler.get_mut(tid) {
        tcb.status = ThreadStatus::Ready;
    }
}
