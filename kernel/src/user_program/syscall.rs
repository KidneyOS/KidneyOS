// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use super::super::threading::RUNNING_THREAD_TID;

use kidneyos_shared::println;

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
        0x1 => {
            // exit
            todo!("exit syscall")
        }
        0x2 => {
            // fork
            todo!("fork syscall")
        }
        0x7 => {
            // waitpid
            todo!("waitpid syscall")
        }
        0x11c => {
            // waitid
            todo!("waitid syscall")
        }
        0xf => {
            // custom syscall
            unsafe {println!("{}", RUNNING_THREAD_TID);}
            0
        }
        _ => 1,
    }
}
