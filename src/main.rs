#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(not(test), no_main)]

#[macro_use]
mod macros;
mod multiboot2;
mod serial;
mod video_memory;

use core::ffi::CStr;
use multiboot2::info::{Info, InfoTag};

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(args: &core::panic::PanicInfo) -> ! {
    eprintln!("{}", args);
    loop {}
}

#[cfg(not(test))]
core::arch::global_asm!(
    "
.globl _start
_start:
        push ebx
        push eax
        call {}",
    sym start,
);

#[allow(dead_code)]
extern "C" fn start(magic: usize, multiboot2_info: *mut Info) -> ! {
    const EXPECTED_MAGIC: usize = 0x36D76289;
    assert!(
        magic == EXPECTED_MAGIC,
        "invalid magic, expected {EXPECTED_MAGIC:#X}, got {magic:#X}"
    );

    // SAFETY: multiboot guarantees that a valid multiboot info pointer will be
    // in ebx when _start is called, and _start puts that on the stack as the
    // second argument which will become the multiboot2_info parameter, so this
    // dereference is safe since we've checked the magic and confirmed we've
    // booted with multiboot.
    let multiboot2_info = unsafe { &mut *multiboot2_info };

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
            InfoTag::BootLoaderName(boot_loader_name_tag) => {
                println!(
                    "Found bootloader name: {:?}",
                    Into::<&CStr>::into(boot_loader_name_tag).to_str()
                )
            }
            InfoTag::BasicMemoryInfo(_) => println!("Found memory info."),
        }
    }

    println!("Done checking info.");

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
