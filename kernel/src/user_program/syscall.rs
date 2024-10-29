// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use crate::fs::syscalls::{
    chdir, close, fstat, ftruncate, getcwd, getdents, link, lseek64, mkdir, mount, open, read,
    rename, rmdir, symlink, sync, unlink, unmount, write,
};
use crate::mem::user::check_and_copy_user_memory;
use crate::mem::util::get_mut_from_user_space;
use crate::system::{running_thread_pid, running_thread_ppid, unwrap_system_mut};
use crate::threading::process::Pid;
use crate::threading::scheduling::{scheduler_yield_and_continue, scheduler_yield_and_die};
use crate::threading::thread_control_block::ThreadControlBlock;
use crate::threading::thread_functions;
use crate::threading::thread_sleep::thread_sleep;
use crate::user_program::elf::Elf;
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
        SYS_OPEN => unsafe { open(arg0 as _, arg1) },
        SYS_READ => unsafe { read(arg0, arg1 as _, arg2 as _) },
        SYS_WRITE => unsafe { write(arg0, arg1 as _, arg2 as _) },
        SYS_LSEEK64 => unsafe { lseek64(arg0, arg1 as _, arg2 as _) },
        SYS_CLOSE => unsafe { close(arg0) },
        SYS_CHDIR => unsafe { chdir(arg0 as _) },
        SYS_GETCWD => unsafe { getcwd(arg0 as _, arg1 as _) },
        SYS_MKDIR => unsafe { mkdir(arg0 as _) },
        SYS_RMDIR => unsafe { rmdir(arg0 as _) },
        SYS_FSTAT => unsafe { fstat(arg0 as _, arg1 as _) },
        SYS_UNLINK => unsafe { unlink(arg0 as _) },
        SYS_GETDENTS => unsafe { getdents(arg0, arg1 as _, arg2 as _) },
        SYS_LINK => unsafe { link(arg0 as _, arg1 as _) },
        SYS_SYMLINK => unsafe { symlink(arg0 as _, arg1 as _) },
        SYS_RENAME => unsafe { rename(arg0 as _, arg1 as _) },
        SYS_FTRUNCATE => unsafe { ftruncate(arg0 as _, arg1 as _, arg2 as _) },
        SYS_UNMOUNT => unsafe { unmount(arg0 as _) },
        SYS_MOUNT => unsafe { mount(arg0 as _, arg1 as _, arg2 as _) },
        SYS_SYNC => sync(),
        SYS_WAITPID => {
            let wait_pid = match arg0 {
                0 => running_thread_pid(),
                _ => arg0 as Pid,
            };

            let status_ptr = match unsafe { get_mut_from_user_space(arg1 as *mut i32) } {
                Some(ptr) => ptr,
                None => return -1,
            };

            let system = unsafe { unwrap_system_mut() };

            let Some(wait_pcb) = system.process.table.get_mut(wait_pid) else {
                return -1;
            }; 

            while wait_pcb.exit_code.is_none() {
                thread_sleep();
            }

            *status_ptr = (wait_pcb.exit_code.unwrap() & 0xff) << 8;
            system.process.table.remove(wait_pid);
            
            wait_pid as isize
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

            println!("{:?}", timespec);
            *timespec_ptr = timespec;
            0
        }
        _ => -ENOSYS,
    }
}
