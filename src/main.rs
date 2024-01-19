#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(not(test), no_main)]

#[macro_use]
mod debug;
mod multiboot2;
#[macro_use]
mod video_memory;

use core::{arch::asm, ffi::CStr};
use multiboot2::info::{Info, InfoTag};

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(args: &core::panic::PanicInfo) -> ! {
    eprintln!("{}", args);
    loop {}
}

#[cfg_attr(target_os = "none", no_mangle)]
pub extern "C" fn _start() -> ! {
    let magic: usize;
    unsafe { asm!("", out("eax") magic) };
    assert!(
        magic == 0x36D76289,
        "invalid magic, expected 0x36D76289, got {:#X}",
        magic
    );

    let multiboot2_info: *mut u32;
    unsafe { asm!("mov {}, ebx", out(reg) multiboot2_info) };
    let multiboot2_info = unsafe { &mut *(multiboot2_info.cast::<Info>()) };

    // TODO: Save the useful information somewhere via copying before we start
    // writing to memory so we don't have to worry about overwriting the
    // multiboot2 info.
    for tag in multiboot2_info.iter() {
        match tag {
            InfoTag::Commandline(commandline_tag) => {
                println!(
                    "Found commandline: {:?}",
                    Into::<&CStr>::into(commandline_tag).to_str()
                )
            }
            InfoTag::BootLoaderName(bootloadoer_name_tag) => {
                println!(
                    "Found bootloader name: {:?}",
                    Into::<&CStr>::into(bootloadoer_name_tag).to_str()
                )
            }
            InfoTag::BasicMemoryInfo(_) => println!("Found memory info."),
        }
    }

    println!("Done checking info.");

    unsafe { asm!("hlt") };
    #[allow(clippy::empty_loop)]
    loop {}
}

#[allow(dead_code)]
const fn add(left: usize, right: usize) -> usize {
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
