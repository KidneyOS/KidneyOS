// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use crate::fs::syscalls::open;
use crate::threading::scheduling::scheduler_yield_and_continue;
use crate::threading::thread_functions;
use kidneyos_shared::println;

pub const SYS_EXIT: usize = 0x1;
pub const SYS_FORK: usize = 0x2;
pub const SYS_READ: usize = 0x3;
pub const SYS_OPEN: usize = 0x5;
pub const SYS_WAITPID: usize = 0x7;
pub const SYS_EXECVE: usize = 0x0b;
pub const SYS_NANOSLEEP: usize = 0xa2;
pub const SYS_SCHED_YIELD: usize = 0x9e;

pub const ENOENT: isize = 2;
pub const EIO: isize = 5;
pub const EBADF: isize = 9;
pub const EFAULT: isize = 14;
pub const EBUSY: isize = 16;
pub const EEXIST: isize = 17;
pub const ENOTDIR: isize = 20;
pub const EISDIR: isize = 21;
pub const EINVAL: isize = 22;
pub const EMFILE: isize = 24;
pub const ENOSPC: isize = 28;
pub const EROFS: isize = 30;
pub const EMLINK: isize = 31;
pub const ENOSYS: isize = 38;
pub const ENOTEMPTY: isize = 39;

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
            todo!("fork syscall")
        }
        SYS_OPEN => unsafe { open(arg0 as _, arg1) as usize },
        SYS_READ => {
            todo!("read syscall")
        }
        SYS_WAITPID => {
            todo!("waitpid syscall")
        }
        SYS_EXECVE => {
            todo!("execv syscall")
        }
        SYS_NANOSLEEP => {
            todo!("nanosleep syscall")
        }
        SYS_SCHED_YIELD => {
            scheduler_yield_and_continue();
            0
        }
        _ => (-ENOSYS) as usize,
    }
}
