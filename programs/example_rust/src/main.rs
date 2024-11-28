#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use kidneyos_syscalls::arguments::RawArguments;

#[no_mangle]
pub extern "C" fn _start(_raw: RawArguments) -> ! {
    kidneyos_syscalls::exit(0);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
