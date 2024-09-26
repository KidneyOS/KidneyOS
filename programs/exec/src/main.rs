#![no_std]
#![no_main]

use core::arch::asm;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        // Exit syscall.
        asm!(
            "
            mov eax, 0x1
            mov ebx, 0x0
            int 0x80
            "
        );
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
