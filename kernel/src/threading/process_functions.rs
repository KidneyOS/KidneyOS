use kidneyos_shared::println;

use crate::system::{running_process_mut, running_thread};

use super::{
    thread_functions::{self, stop_thread},
    thread_sleep::thread_wakeup,
};

pub fn exit_process(exit_code: i32) -> ! {
    let pcb = unsafe { running_process_mut() };
    pcb.exit_code = Some(exit_code);

    if let Some(wait_tid) = pcb.waiting_thread {
        println!("waking wait_tid {}", wait_tid);
        thread_wakeup(wait_tid);
    }

    let running_thread_tid = running_thread().tid;

    // Kill all threads which are part of this process
    pcb.child_tids.iter().for_each(|tid| {
        if *tid != running_thread_tid {
            stop_thread(*tid)
        }
    });

    thread_functions::exit_thread(-1);
}
