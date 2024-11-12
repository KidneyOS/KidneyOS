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
#![feature(inline_const)]

mod block;
mod drivers;
pub mod fs;
mod interrupts;
pub mod mem;
mod paging;
pub mod sync;
mod system;
mod threading;
mod user_program;
pub mod vfs;

extern crate alloc;

use crate::block::block_core::BlockManager;
use crate::drivers::ata::ata_core::ide_init;
use crate::drivers::input::input_core::InputBuffer;
use crate::sync::mutex::Mutex;
use crate::system::{SystemState, SYSTEM};
use crate::threading::process::create_process_state;
use crate::threading::thread_control_block::ThreadControlBlock;
use alloc::boxed::Box;
use core::ptr::NonNull;
use fs::fs_manager::ROOT;
use interrupts::{idt, pic};
use kidneyos_shared::{global_descriptor_table, println, video_memory::VIDEO_MEMORY_WRITER};
use mem::KernelAllocator;
use threading::{create_thread_state, thread_system_start};
use vfs::tempfs::TempFS;

#[cfg_attr(not(test), global_allocator)]
pub static mut KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator::new();

#[cfg(not(test))]
#[panic_handler]
fn panic(args: &core::panic::PanicInfo) -> ! {
    kidneyos_shared::eprintln!("{}", args);
    loop {}
}

const INIT: &[u8] =
    include_bytes!("../../programs/fork/target/i686-unknown-linux-gnu/release/fork").as_slice();

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
        let mut threads = create_thread_state();
        let mut process = create_process_state();
        println!("Finished Thread System initialization. Ready to start threading.");

        let ide_addr = NonNull::new(ide_init as *const () as *mut u8).unwrap();
        let ide_tcb = ThreadControlBlock::new_with_setup(ide_addr, 0, &mut process);

        let block_manager = BlockManager::default();
        let input_buffer = Mutex::new(InputBuffer::new());

        threads.scheduler.push(Box::new(ide_tcb));

        SYSTEM = Some(SystemState {
            threads,
            process,

            block_manager,
            input_buffer,
        });

        println!("Mounting root filesystem...");
        // for now, we just use TempFS for the root filesystem
        ROOT.lock()
            .mount_root(TempFS::new())
            .expect("Couldn't mount root FS");
        println!("Root mounted!");

        thread_system_start(page_manager, INIT);
    }
}
