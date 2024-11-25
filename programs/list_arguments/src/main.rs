#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

use core::ffi::c_char;
use core::mem::MaybeUninit;
use numtoa::NumToA;
use kidneyos_syscalls::arguments::{argument_slice_from_raw, RawArguments};
use kidneyos_syscalls::STDOUT_FILENO;

// Why not use CStr::count_bytes?
//  1. It doesn't seem to be stable yet.
//  2. It seems to invoke strlen, which is obviously not available in our no_str environment.
fn count_bytes(str: *const c_char) -> usize {
    if str.is_null() {
        return 0
    }

    let mut length = 0;

    unsafe {
        while *str.add(length) == b'\0' as c_char {
            length += 1;
        }
    }

    length
}

#[no_mangle]
pub extern "C" fn _start(raw: u32) {
    return
    // kidneyos_syscalls::exit(raw as i32);
    // alloca::with_alloca_zeroed(300, |buffer| {
        // for element in buffer.as_mut() {
            // *element = MaybeUninit::new(0);
        // }

        // Casting out the MaybeUninit.
        // We are trying to avoid a memset emit from rustc, so we are doing a bunch of dancing.
        // let buffer = unsafe {
        //     core::slice::from_raw_parts_mut(buffer.as_mut_ptr().cast::<u8>(), buffer.len())
        // };
        //
    // unsafe {
        // let result = (raw as usize).numtoa(10, &mut GLOBAL_BUFFER);

        // kidneyos_syscalls::write(STDOUT_FILENO, b"Arg: ".as_ptr(), b"Arg: ".len());
        // kidneyos_syscalls::write(STDOUT_FILENO, result.as_ptr(), result.len());
        // kidneyos_syscalls::write(STDOUT_FILENO, b"\n".as_ptr(), 1);
    // }
    // });

    // let arguments = unsafe { argument_slice_from_raw(raw) };
    // 
    // let argument_count_str = b"Argument Count: ";
    // 
    // let argument_start = b"Argument ";
    // let argument_separator = b": ";
    // let argument_newline = b"\n";
    // 
    // kidneyos_syscalls::write(STDOUT_FILENO, argument_count_str.as_ptr(), argument_count_str.len());
    // 
    // let len_char = b'0' + (arguments.len() as u8 % 10);
    // kidneyos_syscalls::write(STDOUT_FILENO, &len_char, 1);
    // kidneyos_syscalls::write(STDOUT_FILENO, argument_newline.as_ptr(), argument_newline.len());
    // 
    // kidneyos_syscalls::write(STDOUT_FILENO, argument_start.as_ptr(), argument_start.len());
    // 
    // for (i, arg) in arguments.into_iter().enumerate() {
    //     kidneyos_syscalls::write(STDOUT_FILENO, argument_start.as_ptr(), argument_start.len());
    // 
    //     // Not going to bother to format numbers.
    //     let i_char = b'0' + (i as u8 % 10);
    //     kidneyos_syscalls::write(STDOUT_FILENO, &i_char, 1);
    //     kidneyos_syscalls::write(STDOUT_FILENO, argument_separator.as_ptr(), argument_separator.len());
    // 
    //     kidneyos_syscalls::write(STDOUT_FILENO, (*arg).cast(), count_bytes(*arg));
    // 
    //     kidneyos_syscalls::write(STDOUT_FILENO, argument_newline.as_ptr(), argument_newline.len());
    // }

    // kidneyos_syscalls::exit(0);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
