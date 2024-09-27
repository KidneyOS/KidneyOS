use super::{
    scheduling::{scheduler_yield_and_block, SCHEDULER},
    thread_control_block::{ThreadStatus, Tid},
};

pub fn thread_sleep() {
    scheduler_yield_and_block();
}

pub fn thread_wakeup(tid: Tid) {
    let scheduler = unsafe { SCHEDULER.as_mut().expect("Scheduler not set up!") };
    if let Some(tcb) = scheduler.get(tid) {
        tcb.status = ThreadStatus::Ready;
    }
}
