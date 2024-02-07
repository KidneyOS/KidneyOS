#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(not(test), no_main)]

mod multiboot2;

extern crate alloc;

use alloc::vec;
use kidneyos::{constants::MB, mem::KernelAllocator, println};
use multiboot2::{
    info::{Info, InfoTag},
    EXPECTED_MAGIC,
};

#[cfg_attr(target_os = "none", global_allocator)]
pub static mut KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator::new();

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(args: &core::panic::PanicInfo) -> ! {
    kidneyos::eprintln!("{}", args);
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
    // TODO: Stack setup.

    assert!(
        magic == EXPECTED_MAGIC,
        "invalid magic, expected {EXPECTED_MAGIC:#X}, got {magic:#X}"
    );

    // SAFETY: multiboot guarantees that a valid multiboot info pointer will be
    // in ebx when _start is called, and _start puts that on the stack as the
    // second argument which will become the multiboot2_info parameter, so this
    // dereference is safe since we've checked the magic and confirmed we've
    // booted with multiboot. Additionally, we drop it before we start writing
    // to anywhere in memory that it might be.
    let mem_upper = unsafe { &mut *multiboot2_info }
        .iter()
        .find_map(|tag| match tag {
            InfoTag::BasicMemoryInfo(t) => Some(t.mem_upper),
            _ => None,
        })
        .expect("Didn't find memory info!");

    // BUG: Ensure the region won't overlap with the kernel code.
    // TODO: The choice of 64MB for kernel memory size should be
    // re-evaluated later.
    // SAFETY: Single core, no interrupts.
    unsafe { KERNEL_ALLOCATOR.init(64 * MB, mem_upper as usize) };

    println!("Allocating vector");
    let mut v = vec![5, 6513, 51];
    println!("Vector ptr: {:?}, capacity: {}", v.as_ptr(), v.capacity());
    assert!(v.pop() == Some(51));
    println!("Dropping vector");
    drop(v);
    println!("Vector dropped!");

    // SAFETY: Single core, no interrupts.
    unsafe { KERNEL_ALLOCATOR.deinit() };

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
