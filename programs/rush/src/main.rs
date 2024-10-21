#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

pub extern "C" fn _start() -> ! {
    kidneyos_syscalls::exit(0);
    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
