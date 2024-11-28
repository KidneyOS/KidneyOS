use crate::system::{running_process, running_thread_tid, unwrap_system};

use super::{
    thread_functions::{self, stop_thread},
    thread_sleep::thread_wakeup,
};

pub fn exit_process(exit_code: i32) -> ! {
    let pcb = running_process();
    let mut pcb = pcb.lock();
    pcb.exit_code = Some(exit_code);

    if let Some(wait_tid) = pcb.waiting_thread {
        thread_wakeup(wait_tid);
    }

    let running_tid = running_thread_tid();

    let system = unwrap_system();
    let mut orphaned_pids = system.process.orhpans.write();

    pcb.child_pids.iter().for_each(|child_pid| {
        if let Some(child_pcb_ref) = system.process.table.get(*child_pid) {
            let mut child_pcb = child_pcb_ref.lock();
            child_pcb.ppid = 0;
            orphaned_pids.push(*child_pid);
        }
    });

    // Kill all threads which are part of this process
    pcb.child_tids.iter().for_each(|tid| {
        if *tid != running_tid {
            stop_thread(*tid)
        }
    });
    drop(pcb);

    thread_functions::exit_thread(-1);
}
