// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use core::time::Duration;

use crate::fs::syscalls::{
    chdir, close, fstat, ftruncate, getcwd, getdents, link, lseek64, mkdir, mount, open, read,
    rename, rmdir, symlink, sync, unlink, unmount, write,
};
use crate::mem::user::check_and_copy_user_memory;
use crate::mem::util::{
    get_mut_from_user_space, get_mut_slice_from_user_space, get_ref_from_user_space,
};
use crate::system::{running_thread_pid, running_thread_ppid, unwrap_system};
use crate::threading::process_functions;
use crate::threading::scheduling::{scheduler_yield_and_continue, scheduler_yield_and_die};
use crate::threading::thread_control_block::ThreadControlBlock;
use crate::user_program::elf::Elf;
use crate::user_program::random::getrandom;
use crate::user_program::time::{get_rtc, get_tsc, Timespec, CLOCK_MONOTONIC, CLOCK_REALTIME};
use alloc::boxed::Box;
use kidneyos_shared::println;
pub use kidneyos_syscalls::defs::*;

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
        SYS_EXECVE => {
            let system = unwrap_system();
            let guard = system.threads.running_thread.lock();
            let thread = guard
                .as_ref()
                .expect("A syscall was called without a running thread.");
            let elf_bytes = check_and_copy_user_memory(arg0, arg1, &thread.page_manager);
            drop(guard);
            let elf = elf_bytes
                .as_ref()
                .and_then(|bytes| Elf::parse_bytes(bytes).ok());

            let Some(elf) = elf else { return -1 };

            let control = ThreadControlBlock::new_from_elf(elf, &system.process);

            system.threads.scheduler.lock().push(Box::new(control));

            scheduler_yield_and_die();
        }
        SYS_GETPID => running_thread_pid() as isize,
        SYS_NANOSLEEP => {
            let Some(input_timespec) = (unsafe { get_ref_from_user_space(arg0 as *mut Timespec) })
            else {
                return -1;
            };

            if input_timespec.tv_sec < 0
                || input_timespec.tv_nsec < 0
                || input_timespec.tv_nsec >= 1_000_000_000
            {
                return -1;
            }

            let target_duration =
                Duration::new(input_timespec.tv_sec as u64, input_timespec.tv_nsec as u32);

            let start_timespec = get_tsc();
            let start_time =
                Duration::new(start_timespec.tv_sec as u64, start_timespec.tv_nsec as u32);
            loop {
                let elapsed_timespec = get_tsc();
                let elapsed_time = Duration::new(
                    elapsed_timespec.tv_sec as u64,
                    elapsed_timespec.tv_nsec as u32,
                ) - start_time;

                if elapsed_time >= target_duration {
                    break;
                }

                scheduler_yield_and_continue();
            }
            0
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
            let Some(buffer) = (unsafe { get_mut_slice_from_user_space(arg0 as *mut u8, arg1) })
            else {
                return -1;
            };

            getrandom(buffer, arg2)
        }
        _ => -ENOSYS,
    }
}
