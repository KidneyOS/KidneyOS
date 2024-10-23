#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]


#[no_mangle]
pub extern "C" fn _start() -> ! {
    let p = kidneyos_syscalls::fork();

    if p == 0 {
        kidneyos_syscalls::
        kidneyos_syscalls::exit(1);
    } else {
        kidneyos_syscalls::exit(100);
    }

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
