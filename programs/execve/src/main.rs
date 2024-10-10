#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

const TARGET_PROGRAM: &[u8] =
    include_bytes!("../../example_rust/target/i686-unknown-linux-gnu/release/example_rust");

#[no_mangle]
pub extern "C" fn _start() -> ! {
    kidneyos_syscalls::execve(TARGET_PROGRAM.as_ptr(), TARGET_PROGRAM.len());

    kidneyos_syscalls::exit(2);

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
