// https://docs.google.com/document/d/1qMMU73HW541wME00Ngl79ou-kQ23zzTlGXJYo9FNh5M

use kidneyos_shared::println;

/// This function is responsible for processing syscalls made by user programs.
/// Its return value is the syscall return value, whose meaning depends on the
/// syscall. It might not actually return sometimes, such as when the syscall
/// is exit.
pub extern "C" fn handler(syscall_number: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    println!("syscall number {syscall_number:#X} with arguments: {arg0:#X} {arg1:#X} {arg2:#X}");
    42
}
