#![feature(allocator_api)]
#![feature(asm_const)]
#![feature(btreemap_alloc)]
#![feature(error_in_core)]
#![feature(naked_functions)]
#![feature(non_null_convenience)]
#![feature(offset_of)]
#![feature(slice_ptr_get)]
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(not(test), no_main)]
#![feature(negative_impls)]

mod block;
mod drivers;
mod interrupts;
mod mem;
mod paging;
mod sync;
mod threading;
mod user_program;

extern crate alloc;

use crate::block::block_core::block_init;
use crate::drivers::ata::ata_core::ide_init;
use interrupts::{idt, pic};
use kidneyos_shared::{global_descriptor_table, println, video_memory::VIDEO_MEMORY_WRITER};
use mem::KernelAllocator;
use threading::{thread_system_initialization, thread_system_start};

#[cfg_attr(target_os = "none", global_allocator)]
pub static mut KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator::new();

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(args: &core::panic::PanicInfo) -> ! {
    kidneyos_shared::eprintln!("{}", args);
    loop {}
}

const INIT: &[u8] = include_bytes!("../../programs/exit/exit").as_slice();

#[cfg_attr(not(test), no_mangle)]
extern "C" fn main(mem_upper: usize, video_memory_skip_lines: usize) -> ! {
    unsafe {
        VIDEO_MEMORY_WRITER.skip_lines(video_memory_skip_lines);
    }

    // SAFETY: Single core, interrupts disabled.
    unsafe {
        KERNEL_ALLOCATOR.init(mem_upper);

        println!("Setting up IDTR");
        idt::load();
        println!("IDTR set up!");

        println!("Enabling paging");
        let page_manager = paging::enable();
        println!("Paging enabled!");

        println!("Setting up GDTR");
        global_descriptor_table::load();
        println!("GDTR set up!");

        println!("Setting up PIT");
        pic::pic_remap(pic::PIC1_OFFSET, pic::PIC2_OFFSET);
        pic::init_pit();
        println!("PIT set up!");

        println!("Setting up block layer");
        let block_manager = block_init();
        println!("Block layer set up!");

        println!("Initializing Thread System...");
        thread_system_initialization();
        println!("Finished Thread System initialization. Ready to start threading.");

        println!("Setting up IDE");
        let _block_manager = ide_init(block_manager, true);
        println!("IDE set up!");

        thread_system_start(page_manager, INIT);
    }
}
