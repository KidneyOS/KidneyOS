#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use core::ffi::c_char;
use kidneyos_syscalls::arguments::RawArguments;
use kidneyos_syscalls::O_CREATE;

const TARGET_PROGRAM: &[u8] =
    include_bytes!("../../list_arguments/target/i686-unknown-linux-gnu/release/list_arguments");

const TARGET_PATH: *const c_char = c"/example_rust".as_ptr();

#[no_mangle]
pub extern "C" fn _start(_arguments: RawArguments) {
    // TempFS - We'll create the file that we want to execute on the fly.
    let fd = kidneyos_syscalls::open(TARGET_PATH, O_CREATE);

    if fd < 0 {
        kidneyos_syscalls::exit(fd);
    }

    let result = kidneyos_syscalls::write(fd, TARGET_PROGRAM.as_ptr(), TARGET_PROGRAM.len());

    if result < 0 {
        kidneyos_syscalls::exit(result);
    }

    // Flush?
    kidneyos_syscalls::close(fd);

    let argv = [TARGET_PATH, core::ptr::null()];

    let envp = [core::ptr::null()];

    let result = kidneyos_syscalls::execve(TARGET_PATH, argv.as_ptr(), envp.as_ptr());

    kidneyos_syscalls::exit(result)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
