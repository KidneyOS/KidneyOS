#![feature(naked_functions)]
#![feature(asm_const)]
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(not(test), no_main)]

mod global_descriptor_table;
mod interrupt_descriptor_table;
mod multiboot2;
mod paging;

extern crate alloc;

use kidneyos::{
    constants::MB, mem::KERNEL_ALLOCATOR, println, threading::thread_system_initialization,
};
use multiboot2::{
    info::{Info, InfoTag},
    EXPECTED_MAGIC,
};

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
        mov esp, {stack_start}
        push ebx
        push eax
        call {}",
    sym start,
    stack_start = const kidneyos::mem::KERNEL_MAIN_STACK_TOP,
);

#[allow(dead_code)]
extern "C" fn start(magic: usize, multiboot2_info: *mut Info) -> ! {
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
    let kernel_memory_range = unsafe { KERNEL_ALLOCATOR.init(64 * MB, mem_upper as usize) };

    println!("Setting up GDTR");
    unsafe { global_descriptor_table::load() };
    println!("GDTR set up!");

    println!("Setting up IDTR");
    unsafe { interrupt_descriptor_table::load() };
    println!("IDTR set up!");

    println!("Enabling paging");
    unsafe { paging::enable(kernel_memory_range) };
    println!("Paging enabled!");

    thread_system_initialization();

    // SAFETY: Single core, no interrupts.
    // unsafe { KERNEL_ALLOCATOR.deinit() };

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
