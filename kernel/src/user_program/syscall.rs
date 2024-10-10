// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use crate::mem::user::check_and_copy_user_memory;
use crate::threading::scheduling::{
    scheduler_yield_and_continue, scheduler_yield_and_die, SCHEDULER,
};
use crate::threading::thread_control_block::ThreadControlBlock;
use crate::threading::{thread_functions, RUNNING_THREAD};
use crate::user_program::elf::Elf;
use alloc::boxed::Box;
use kidneyos_shared::println;

pub const SYS_EXIT: usize = 0x1;
pub const SYS_FORK: usize = 0x2;
pub const SYS_READ: usize = 0x3;
pub const SYS_WAITPID: usize = 0x7;
pub const SYS_EXECVE: usize = 0x0b;
pub const SYS_NANOSLEEP: usize = 0xa2;
pub const SYS_SCHED_YIELD: usize = 0x9e;

/// This function is responsible for processing syscalls made by user programs.
/// Its return value is the syscall return value, whose meaning depends on the syscall.
/// It might not actually return sometimes, such as when the syscall is exit.
pub extern "C" fn handler(syscall_number: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
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
            todo!("fork syscall")
        }
        SYS_READ => {
            todo!("read syscall")
        }
        SYS_WAITPID => {
            todo!("waitpid syscall")
        }
        SYS_EXECVE => {
            let thread = unsafe {
                RUNNING_THREAD
                    .as_ref()
                    .expect("A syscall was called without a running thread.")
            };

            let scheduler = unsafe {
                SCHEDULER
                    .as_mut()
                    .expect("A syscall was called without a scheduler.")
            };

            let elf_bytes = check_and_copy_user_memory(arg0, arg1, &thread.page_manager);
            let elf = elf_bytes
                .as_ref()
                .and_then(|bytes| Elf::parse_bytes(bytes).ok());

            let Some(elf) = elf else { return -1 };

            let control = ThreadControlBlock::new_from_elf(elf);

            scheduler.push(Box::new(control));

            scheduler_yield_and_die();
        }
        SYS_NANOSLEEP => {
            todo!("nanosleep syscall")
        }
        SYS_SCHED_YIELD => {
            scheduler_yield_and_continue();
            0
        }
        _ => 1,
    }
}
