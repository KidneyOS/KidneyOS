use crate::system::{running_process_mut, running_thread};

use super::{
    thread_functions::{self, stop_thread},
    thread_sleep::thread_wakeup,
};

pub fn exit_process(exit_code: i32) -> ! {
    let mut pcb = running_process_mut();
    pcb.exit_code = Some(exit_code);

    if let Some(wait_tcb) = &pcb.waiting_thread {
        thread_wakeup(wait_tcb.borrow().tid);
    }

    let running_thread_tid = running_thread().tid;

    // Kill all threads which are part of this process
    pcb.child_tcbs
        .iter()
        .filter(|tcb| tcb.borrow().tid != running_thread_tid)
        .for_each(|tcb| {
            stop_thread(tcb.clone());
        });

    thread_functions::exit_thread(-1);
}
