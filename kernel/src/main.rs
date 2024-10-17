#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![feature(allocator_api)]
#![feature(asm_const)]
#![feature(btreemap_alloc)]
#![feature(error_in_core)]
#![feature(naked_functions)]
#![feature(non_null_convenience)]
#![feature(offset_of)]
#![feature(slice_ptr_get)]
#![feature(negative_impls)]
#![feature(pointer_is_aligned)]

mod block;
mod drivers;
mod interrupts;
pub mod mem;
mod paging;
mod sync;
mod threading;
mod user_program;
pub mod vfs;

extern crate alloc;

use crate::drivers::ata::ata_core::ide_init;
use crate::threading::scheduling::SCHEDULER;
use crate::threading::thread_control_block::ThreadControlBlock;
use alloc::boxed::Box;
use core::ptr::NonNull;
use interrupts::{idt, pic};
use kidneyos_shared::{global_descriptor_table, println, video_memory::VIDEO_MEMORY_WRITER};
use mem::KernelAllocator;
use threading::{thread_system_initialization, thread_system_start};

#[cfg_attr(not(test), global_allocator)]
pub static mut KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator::new();

#[cfg(not(test))]
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

        println!("Initializing Thread System...");
        thread_system_initialization();
        println!("Finished Thread System initialization. Ready to start threading.");

        let ide_addr = NonNull::new(ide_init as *const () as *mut u8).unwrap();
        let ide_tcb = ThreadControlBlock::new_with_setup(ide_addr, 0);

        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(Box::new(ide_tcb));

        thread_system_start(page_manager, INIT);
    }
}
