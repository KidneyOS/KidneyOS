#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use kidneyos_syscalls::EPIPE;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut pipes = [0, 0];

    kidneyos_syscalls::pipe(pipes.as_mut_ptr());

    let read = pipes[0];
    let write = pipes[1];

    let other_read = kidneyos_syscalls::dup(read);

    let data: [u8; 5] = [1, 2, 3, 4, 5];

    let error = kidneyos_syscalls::write(write, data.as_ptr(), data.len());

    if error < 0 {
        kidneyos_syscalls::exit(error);
    }

    let mut buf: [u8; 3] = [0, 0, 0];

    let bytes = kidneyos_syscalls::read(read, buf.as_mut_ptr(), buf.len());

    if bytes != 3 {
        kidneyos_syscalls::exit(bytes);
    }

    if buf != [1, 2, 3] {
        kidneyos_syscalls::exit(0x100);
    }

    let bytes = kidneyos_syscalls::read(other_read, buf.as_mut_ptr(), buf.len());

    if bytes != 2 {
        kidneyos_syscalls::exit(bytes);
    }

    if buf[0] != 4 || buf[1] != 5 {
        kidneyos_syscalls::exit(0x200);
    }

    kidneyos_syscalls::close(read);

    // Should not be EPIPE
    if kidneyos_syscalls::write(write, data.as_ptr(), data.len()) == -EPIPE as i32 {
        kidneyos_syscalls::exit(0x300);
    }

    kidneyos_syscalls::close(other_read);

    // Should be EPIPE
    if kidneyos_syscalls::write(write, data.as_ptr(), data.len()) != -EPIPE as i32 {
        kidneyos_syscalls::exit(0x400);
    }

    kidneyos_syscalls::exit(0);

    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
