// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use crate::mem::util::get_mut_from_user_space;
use crate::threading::process_table::PROCESS_TABLE;
use crate::threading::scheduling::{scheduler_yield_and_block, scheduler_yield_and_continue, SCHEDULER};
use crate::threading::thread_control_block::{Pid, ThreadStatus};
use crate::threading::{thread_functions, RUNNING_THREAD};
use crate::user_program::time::{current_time, Timespec};
use kidneyos_shared::println;

/// This function is responsible for processing syscalls made by user programs.
/// Its return value is the syscall return value, whose meaning depends on the syscall.
/// It might not actually return sometimes, such as when the syscall is exit.
pub extern "C" fn handler(syscall_number: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    println!("syscall number {syscall_number:#X} with arguments: {arg0:#X} {arg1:#X} {arg2:#X}");
    // TODO: Start implementing this by branching on syscall_number.
    // Add todo!()'s for any syscalls that aren't implemented.
    // Return an error if an invalid syscall number is provided.
    // Translate between syscall names and numbers: https://x86.syscall.sh/
    match syscall_number {
        SYS_EXIT => {
            thread_functions::exit_thread(arg0 as i32);
        }
        SYS_FORK => {
            todo!("fork not implemented");
            // // TODO: fix the virtual address already allocated error
            // let running_tcb = unsafe { RUNNING_THREAD.as_ref().expect("Why is nothing Running!?") };
            // let parent_tid = running_tcb.tid;
            //
            // // TODO: fix cloning of TCB
            // let child_tcb = (**running_tcb).clone();
            // let child_tid = child_tcb.tid as usize;
            //
            // if parent_tid == running_tcb.tid {
            //     child_tid
            // } else {
            //     // Still gettng an error that the page table is being dropped while loaded here
            //     println!("{}", child_tcb.page_manager.is_loaded());
            //     intr_disable();
            //     unsafe {
            //         SCHEDULER
            //             .as_mut()
            //             .expect("Scheduler not set up!").push(Box::new(child_tcb))
            //     };
            //     intr_enable();
            //     0
            // }
        }
        SYS_READ => {
            println!("(syscall) starting read");

            scheduler_yield_and_block();

            // Should only reach here if read is completed
            println!("(syscall) completed read");
            2048
        }
        SYS_WAITPID => {
            todo!("waitpid syscall");
        }
        SYS_EXECVE => {
            todo!("execv syscall");
        }
        SYS_KILL => {
            if arg1 != 0 {
                panic!("Signals not implemented yet.");
            }

            if arg2 == 0 {
                panic!("Process groups not implemented yet.")
            }

            let process_table = unsafe { PROCESS_TABLE.as_mut().expect("No process table set up").as_mut() };
            if let Some(pcb) = process_table.get(arg0 as Pid) {
                let scheduler = unsafe { SCHEDULER.as_mut().expect("No scheduler set up").as_mut() };
                pcb.child_tids.iter().for_each(|tid| scheduler.get_mut(*tid).unwrap().status = ThreadStatus::Dying);
            } else {
                return usize::MAX;
            }
            
            0
        }
        SYS_NANOSLEEP => {
            todo!("nanosleep syscall");
        }
        SYS_SCHED_YIELD => {
            scheduler_yield_and_continue();
            0
        }
        SYS_GETPID => {
            let tcb = unsafe { RUNNING_THREAD.as_mut().expect("Why is nothing running?") };
            tcb.pid as usize
        }
        SYS_CLOCK_GETTIME => {
            if arg0 != 0 {
                return usize::MAX;
            }

            let timespec_ptr= match unsafe { get_mut_from_user_space(arg1 as *mut Timespec) } {
                Some(ptr) => ptr,
                None => return usize::MAX,
            };

            let timespec = current_time();
            *timespec_ptr = timespec;
            0
        }
        _ => 1,
    }
}

pub const SYS_EXIT: usize = 0x1;
pub const SYS_FORK: usize = 0x2;
pub const SYS_READ: usize = 0x3;
pub const SYS_WAITPID: usize = 0x7;
pub const SYS_EXECVE: usize = 0xb;
pub const SYS_KILL: usize = 0x25;
pub const SYS_NANOSLEEP: usize = 0xa2;
pub const SYS_SCHED_YIELD: usize = 0x9e;
pub const SYS_GETPID: usize = 0x14;
pub const SYS_CLOCK_GETTIME: usize = 0x109;
