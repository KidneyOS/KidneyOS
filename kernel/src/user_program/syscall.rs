// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use crate::threading::scheduling::{
    scheduler_yield_and_block, scheduler_yield_and_continue, SCHEDULER,
};
use crate::threading::{thread_functions, RUNNING_THREAD};
use alloc::boxed::Box;
use core::arch::asm;
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
            // TODO: fix the virtual address already allocated error
            let running_tcb = unsafe { RUNNING_THREAD.as_ref().expect("Why is nothing Running!?") };
            let parent_tid = running_tcb.tid;

            let child_tcb = (**running_tcb).clone();
            let child_tid = child_tcb.tid as usize;

            unsafe {
                SCHEDULER
                    .as_mut()
                    .expect("Scheduler not set up!")
                    .push(Box::new(child_tcb))
            };

            if parent_tid == running_tcb.tid {
                child_tid
            } else {
                0
            }
        }
        SYS_READ => {
            println!("(syscall) starting read");

            unsafe {
                timer();
            }

            scheduler_yield_and_block();

            // Should only reach here if read is completed
            println!("(syscall) completed read");
            2048
        }
        SYS_WAITPID => {
            todo!("waitpid syscall")
        }
        SYS_EXECVE => {
            // todo!("exec syscall")
            // let running_tcb = unsafe { RUNNING_THREAD.as_ref().expect("Why is nothing Running!?") };
            // Hard code the elf file to load for now

            // Should only reach here if there is an error
            1
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

#[allow(unused)]
pub unsafe extern "C" fn timer() {
    println!("(interrupt) caught timer!");
    let mut count: usize = 100;
    asm!(
        "
        in {count}, 0x40
        mov ebx, {count}
        test eax, ebx
        jz $2f
        dec eax
        mov {count}, eax
2:
        ",
        count = out(reg) count
    );
}

pub const SYS_EXIT: usize = 0x1;
pub const SYS_FORK: usize = 0x2;
pub const SYS_READ: usize = 0x3;
pub const SYS_WAITPID: usize = 0x7;
pub const SYS_EXECVE: usize = 0xb;
pub const SYS_NANOSLEEP: usize = 0xa2;
pub const SYS_SCHED_YIELD: usize = 0x9e;
pub const SYS_GETPID: usize = 0x14;
