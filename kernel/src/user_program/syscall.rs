// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use crate::threading::scheduling::{
    scheduler_yield_and_block, scheduler_yield_and_continue, SCHEDULER,
};
use crate::threading::thread_control_block::PROCESS_TABLE;
use crate::threading::thread_sleep::thread_sleep;
use crate::threading::{thread_functions, RUNNING_THREAD};
use alloc::boxed::Box;
use core::slice::SliceIndex;
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
            // todo!("waitpid not implemented");
            println!("Starting wait syscall");

            if arg0 == 0 {
                todo!("process groups not implemented");
            }

            let running_tcb = unsafe { RUNNING_THREAD.as_ref().expect("Why is nothing Running!?") };

            let process_table = unsafe { PROCESS_TABLE.as_mut().expect("No process table set up").as_mut() };
            if let Some(parnet_pcb) = process_table.get_mut(arg0 as u16) {
                parnet_pcb.wait_list.push(running_tcb.pid);

                thread_sleep();

                parnet_pcb.pid as usize
            } else {
                // Parent TID not found
                usize::max_value()
            }
        }
        SYS_EXECVE => {
            // todo!("exec syscall");
            // let running_tcb = unsafe { RUNNING_THREAD.as_ref().expect("Why is nothing Running!?") };
            // Hard code the elf file to load for now

            // Should only reach here if there is an error
            0
        }
        SYS_NANOSLEEP => {
            todo!("nanosleep syscall")
        }
        SYS_SCHED_YIELD => {
            scheduler_yield_and_continue();
            0
        }
        SYS_GETPID => {
            let tcb = unsafe { RUNNING_THREAD.as_mut().expect("Why is nothing running?") };
            tcb.pid as usize
        }
        _ => 1,
    }
}

pub const SYS_EXIT: usize = 0x1;
pub const SYS_FORK: usize = 0x2;
pub const SYS_READ: usize = 0x3;
pub const SYS_WAITPID: usize = 0x7;
pub const SYS_EXECVE: usize = 0xb;
pub const SYS_NANOSLEEP: usize = 0xa2;
pub const SYS_SCHED_YIELD: usize = 0x9e;
pub const SYS_GETPID: usize = 0x14;
