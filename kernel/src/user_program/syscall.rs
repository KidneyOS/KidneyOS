// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use crate::fs::syscalls::{
    chdir, close, fstat, ftruncate, getcwd, getdents, link, lseek64, mkdir, mount, open, read,
    rename, rmdir, symlink, sync, unlink, unmount, write,
};
use crate::mem::util::get_mut_from_user_space;
use crate::system::{running_thread_pid, running_thread_ppid};
use crate::threading::process_functions;
use crate::threading::scheduling::scheduler_yield_and_continue;
use crate::user_program::process::execve;
use crate::user_program::random::getrandom;
use crate::user_program::time::{get_rtc, get_tsc, Timespec, CLOCK_MONOTONIC, CLOCK_REALTIME};
use core::slice::from_raw_parts_mut;
use kidneyos_shared::println;
pub use kidneyos_syscalls::defs::*;

/// This function is responsible for processing syscalls made by user programs.
/// Its return value is the syscall return value, whose meaning depends on the syscall.
/// It might not actually return sometimes, such as when the syscall is exit.
pub extern "C" fn handler(syscall_number: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    // println!("syscall number {syscall_number:#X} with arguments: {arg0:#X} {arg1:#X} {arg2:#X}");
    // TODO: Start implementing this by branching on syscall_number.
    // Add todo!()'s for any syscalls that aren't implemented.
    // Return an error if an invalid syscall number is provided.
    // Translate between syscall names and numbers: https://x86.syscall.sh/
    match syscall_number {
        SYS_EXIT => {
            println!("Exiting with code: {arg0}");
            process_functions::exit_process(arg0 as i32);
        }
        SYS_FORK => {
            todo!("fork syscall")
        }
        SYS_OPEN => open(arg0 as _, arg1),
        SYS_READ => read(arg0, arg1 as _, arg2 as _),
        SYS_WRITE => write(arg0, arg1 as _, arg2 as _),
        SYS_LSEEK64 => lseek64(arg0, arg1 as _, arg2 as _),
        SYS_CLOSE => close(arg0),
        SYS_CHDIR => chdir(arg0 as _),
        SYS_GETCWD => getcwd(arg0 as _, arg1 as _),
        SYS_MKDIR => mkdir(arg0 as _),
        SYS_RMDIR => rmdir(arg0 as _),
        SYS_FSTAT => fstat(arg0 as _, arg1 as _),
        SYS_UNLINK => unlink(arg0 as _),
        SYS_GETDENTS => getdents(arg0, arg1 as _, arg2 as _),
        SYS_LINK => link(arg0 as _, arg1 as _),
        SYS_SYMLINK => symlink(arg0 as _, arg1 as _),
        SYS_RENAME => rename(arg0 as _, arg1 as _),
        SYS_FTRUNCATE => ftruncate(arg0 as _, arg1 as _, arg2 as _),
        SYS_UNMOUNT => unmount(arg0 as _),
        SYS_MOUNT => mount(arg0 as _, arg1 as _, arg2 as _),
        SYS_SYNC => sync(),
        SYS_WAITPID => {
            todo!("waitpid syscall")
        }
        SYS_EXECVE => execve(arg0 as _, arg1 as _, arg2 as _),
        SYS_GETPID => running_thread_pid() as isize,
        SYS_NANOSLEEP => {
            todo!("nanosleep syscall")
        }
        SYS_GETPPID => running_thread_ppid() as isize,
        SYS_SCHED_YIELD => {
            scheduler_yield_and_continue();
            0
        }
        SYS_CLOCK_GETTIME => {
            let timespec = match arg0 {
                CLOCK_REALTIME => get_rtc(),
                CLOCK_MONOTONIC => get_tsc(),
                _ => return -1, // Only supporting realtime and monotonic for now
            };

            let Some(timespec_ptr) = (unsafe { get_mut_from_user_space(arg1 as *mut Timespec) })
            else {
                return -1;
            };

            *timespec_ptr = timespec;
            0
        }
        SYS_GETRANDOM => {
            let Some(buffer_ptr) = (unsafe { get_mut_from_user_space(arg0 as *mut u8) }) else {
                return -1;
            };

            let buffer = unsafe { from_raw_parts_mut(buffer_ptr, arg1) };
            getrandom(buffer, arg1, arg2)
        }
        _ => -ENOSYS,
    }
}
