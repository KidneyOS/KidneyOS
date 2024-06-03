// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use kidneyos_shared::println;
use crate::threading::{thread_functions, scheduling};
use crate::threading::thread_control_block::ThreadControlBlock;

/// This function is responsible for processing syscalls made by user programs.
/// Its return value is the syscall return value, whose meaning depends on the
/// syscall. It might not actually return sometimes, such as when the syscall
/// is exit.
pub extern "C" fn handler(syscall_number: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    println!("syscall number {syscall_number:#X} with arguments: {arg0:#X} {arg1:#X} {arg2:#X}");
    // TODO: Start implementing this by branching on syscall_number. Add
    // todo!()'s for any syscalls that aren't implemented. Return an error if an
    // invalid syscall number is provided.
    // Translate between syscall names and numbers: https://x86.syscall.sh/
    match syscall_number {
        SYS_EXIT => {
            thread_functions::exit_thread(arg0 as i32);
        }
        SYS_FORK => {
            // Need to get the elf data
            let sibling: ThreadControlBlock = ThreadControlBlock::new();
            scheduling::scheduler_yield_and_continue();
            sibling.tid as usize
        }
        0x7 => {
            todo!("waitpid syscall")
        }
        0x11c => {
            todo!("waitid syscall")
        }
        _ => 1,
    }
}

pub const SYS_EXIT: usize = 0x1;
pub const SYS_FORK: usize = 0x2;
