#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]


#[no_mangle]
pub extern "C" fn _start() -> ! {
    let p = kidneyos_syscalls::fork();

    if p == 0 {
        kidneyos_syscalls::exit(1);
    } else {
        let mut status: i32 = 0;
        kidneyos_syscalls::waitpid(p, &mut status, 0);
        if kidneyos_syscalls::wifexited(status) {
            kidneyos_syscalls::exit(2);
        }

        let exit_code = kidneyos_syscalls::wifexitstatus(status) as usize;
        // Should be 1
        kidneyos_syscalls::exit(exit_code);
    }

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
