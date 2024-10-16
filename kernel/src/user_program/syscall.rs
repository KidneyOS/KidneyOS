// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use crate::fs::syscalls::{
    chdir, close, fstat, ftruncate, getcwd, getdents, link, lseek64, mkdir, mount, open, read,
    rename, rmdir, symlink, sync, unlink, unmount, write,
};
use crate::threading::scheduling::scheduler_yield_and_continue;
use crate::threading::thread_functions;
use kidneyos_shared::println;
pub use kidneyos_syscalls::defs::*;

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
        SYS_READ => unsafe { read(arg0, arg1 as _, arg2 as _) as usize },
        SYS_WRITE => unsafe { write(arg0, arg1 as _, arg2 as _) as usize },
        SYS_LSEEK64 => unsafe { lseek64(arg0, arg1 as _, arg2 as _) as usize },
        SYS_CLOSE => unsafe { close(arg0) as usize },
        SYS_CHDIR => unsafe { chdir(arg0 as _) as usize },
        SYS_GETCWD => unsafe { getcwd(arg0 as _, arg1 as _) as usize },
        SYS_MKDIR => unsafe { mkdir(arg0 as _) as usize },
        SYS_RMDIR => unsafe { rmdir(arg0 as _) as usize },
        SYS_FSTAT => unsafe { fstat(arg0 as _, arg1 as _) as usize },
        SYS_UNLINK => unsafe { unlink(arg0 as _) as usize },
        SYS_GETDENTS => unsafe { getdents(arg0, arg1 as _, arg2 as _) as usize },
        SYS_LINK => unsafe { link(arg0 as _, arg1 as _) as usize },
        SYS_SYMLINK => unsafe { symlink(arg0 as _, arg1 as _) as usize },
        SYS_RENAME => unsafe { rename(arg0 as _, arg1 as _) as usize },
        SYS_FTRUNCATE => unsafe { ftruncate(arg0 as _, arg1 as _, arg2 as _) as usize },
        SYS_UNMOUNT => unsafe { unmount(arg0 as _) as usize },
        SYS_MOUNT => unsafe { mount(arg0 as _, arg1 as _, arg2 as _) as usize },
        SYS_SYNC => sync() as usize,
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
