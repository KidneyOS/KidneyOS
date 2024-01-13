#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(not(test), no_main)]

mod multiboot2_header;

use core::arch::asm;

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg_attr(target_os = "none", no_mangle)]
fn _start() {
    #[allow(unused)]
    #[repr(packed)]
    struct Character {
        ascii: u8,
        attribute: u8,
    }

    const VIDEO_MEMORY_BASE: usize = 0xb8000;
    let video_memory =
        unsafe { core::slice::from_raw_parts_mut(VIDEO_MEMORY_BASE as *mut Character, 80 * 25) };

    let mut print = |start: usize, bytes: &[u8]| {
        for (i, b) in bytes.into_iter().enumerate() {
            video_memory[start + i] = Character {
                ascii: *b,
                attribute: 0x2f,
            };
        }
    };

    static HELLO: &[u8] = b"Hello, world!";
    print(0, HELLO);

    unsafe { asm!("hlt") };
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
