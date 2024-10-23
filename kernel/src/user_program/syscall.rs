// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use crate::mem::user::check_and_copy_user_memory;
use crate::system::{running_thread_pid, running_thread_ppid, unwrap_system_mut};
use crate::threading::scheduling::{scheduler_yield_and_continue, scheduler_yield_and_die};
use crate::threading::thread_control_block::ThreadControlBlock;
use crate::threading::thread_functions;
use crate::user_program::elf::Elf;
use alloc::boxed::Box;
use kidneyos_shared::println;

pub const SYS_EXIT: usize = 0x1;
pub const SYS_FORK: usize = 0x2;
pub const SYS_READ: usize = 0x3;
pub const SYS_WAITPID: usize = 0x7;
pub const SYS_EXECVE: usize = 0x0b;
pub const SYS_GETPID: usize = 0x14;
pub const SYS_NANOSLEEP: usize = 0xa2;
pub const SYS_GETPPID: usize = 0x40;
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
            let system = unsafe { unwrap_system_mut() };
            let running_thread = system.threads.running_thread.as_ref().unwrap();

            let child_tcb = running_thread.new_from_fork(&mut system.process);
            let child_pid = child_tcb.pid;

            system.threads.scheduler.push(Box::new(child_tcb));

            match running_thread_pid() {
                pid if pid == child_pid => 0,
                _ => child_pid as isize,
            }
        }
        SYS_READ => {
            todo!("read syscall")
        }
        SYS_WAITPID => {
            todo!("waitpid syscall")
        }
        SYS_EXECVE => {
            let thread = unsafe {
                unwrap_system_mut()
                    .threads
                    .running_thread
                    .as_ref()
                    .expect("A syscall was called without a running thread.")
            };

            let elf_bytes = check_and_copy_user_memory(arg0, arg1, &thread.page_manager);
            let elf = elf_bytes
                .as_ref()
                .and_then(|bytes| Elf::parse_bytes(bytes).ok());

            let Some(elf) = elf else { return -1 };

            let system = unsafe { unwrap_system_mut() };
            let control = ThreadControlBlock::new_from_elf(elf, &mut system.process);

            unsafe {
                unwrap_system_mut()
                    .threads
                    .scheduler
                    .push(Box::new(control));
            }

            scheduler_yield_and_die();
        }
        SYS_GETPID => running_thread_pid() as isize,
        SYS_NANOSLEEP => {
            todo!("nanosleep syscall")
        }
        SYS_GETPPID => running_thread_ppid() as isize,
        SYS_SCHED_YIELD => {
            scheduler_yield_and_continue();
            0
        }
        _ => 1,
    }
}
