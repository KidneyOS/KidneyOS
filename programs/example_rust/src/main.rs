#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    kidneyos_syscalls::exit(1);
    
    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
