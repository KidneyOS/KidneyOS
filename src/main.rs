#![feature(allocator_api)]
#![feature(btreemap_alloc)]
#![feature(naked_functions)]
#![feature(non_null_convenience)]
#![feature(slice_ptr_get)]
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(not(test), no_main)]

mod interrupt_descriptor_table;
mod mem;
mod paging;
#[allow(unused)]
mod threading;

extern crate alloc;

use crate::threading::thread_system_initialization;
use kidneyos_core::{println, video_memory::VIDEO_MEMORY_WRITER};
use mem::KernelAllocator;

#[cfg_attr(target_os = "none", global_allocator)]
pub static mut KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator::new();

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(args: &core::panic::PanicInfo) -> ! {
    kidneyos_core::eprintln!("{}", args);
    loop {}
}

#[cfg_attr(not(test), no_mangle)]
extern "C" fn main(mem_upper: usize, video_memory_skip_lines: usize) -> ! {
    unsafe {
        VIDEO_MEMORY_WRITER.skip_lines(video_memory_skip_lines);
    }

    // SAFETY: Single core, interrupts disabled.
    unsafe {
        KERNEL_ALLOCATOR.init(mem_upper);

        println!("Setting up IDTR");
        interrupt_descriptor_table::load();
        println!("IDTR set up!");

        println!("Enabling paging");
        paging::enable();
        println!("Paging enabled!");

        thread_system_initialization();
    }

    #[allow(clippy::empty_loop)]
    loop {}
}
