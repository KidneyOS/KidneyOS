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
use crate::fs::fs_manager::RootFileSystem;
use crate::sync::mutex::Mutex;
use crate::sync::rwlock::sleep::RwLock;
use crate::system::SystemState;
use crate::threading::process::create_process_state;
use crate::threading::thread_control_block::ThreadControlBlock;
use alloc::boxed::Box;
use core::ptr::NonNull;
use interrupts::{idt, pic};
use kidneyos_shared::{global_descriptor_table, print, println, video_memory::VIDEO_MEMORY_WRITER};
use mem::KernelAllocator;
use threading::{create_thread_state, thread_system_start};
use vfs::tempfs::TempFS;
use crate::sync::semaphore::Semaphore;

#[cfg_attr(not(test), global_allocator)]
pub static mut KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator::new();

#[cfg(not(test))]
#[panic_handler]
fn panic(args: &core::panic::PanicInfo) -> ! {
    kidneyos_shared::eprintln!("{}", args);
    loop {}
}

const INIT: &[u8] = include_bytes!("../../programs/exit/exit").as_slice();

#[no_mangle]
extern "C" fn thread_one(semaphore: &Semaphore) {
    println!("Thread One Start");

    {
        let _permit = semaphore.acquire();

        println!("Thread One has semaphore... holding");
        for i in 0 .. 10000000 {

        }
    }

    println!("Thread one released semaphore!");
}

#[no_mangle]
extern "C" fn thread_two(semaphore: &Semaphore) {
    println!("Thread Two Start");

    {
        let _permit = semaphore.acquire();

        println!("Thread Two has semaphore... holding");
        for i in 0 .. 10000000 {

        }
    }

    println!("Thread two released semaphore!");
}

#[no_mangle]
extern "C" fn thread_three(semaphore: &Semaphore) {
    println!("Thread Three Start");

    {
        let _permit = semaphore.acquire();

        println!("Thread three has semaphore... holding");
        for i in 0 .. 10000000 {

        }
    }

    println!("Thread three released semaphore!");

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
        let threads = create_thread_state();
        let mut process = create_process_state();
        println!("Finished Thread System initialization. Ready to start threading.");

        let semaphore = Semaphore::new(2);

        let my_fn_one = NonNull::new(thread_one as *const () as *mut u8).unwrap();
        let mut my_tcb_one = ThreadControlBlock::new_with_setup(my_fn_one, true, (&semaphore) as *const Semaphore as usize as u32, &mut process);

        let my_fn_two = NonNull::new(thread_two as *const () as *mut u8).unwrap();
        let mut my_tcb_two = ThreadControlBlock::new_with_setup(my_fn_two, true, (&semaphore) as *const Semaphore as usize as u32, &mut process);

        let my_fn_three = NonNull::new(thread_three as *const () as *mut u8).unwrap();
        let mut my_tcb_three = ThreadControlBlock::new_with_setup(my_fn_three, true, (&semaphore) as *const Semaphore as usize as u32, &mut process);

        // my_tcb.push_argument(1u32).unwrap();

        // let ide_addr = NonNull::new(ide_init as *const () as *mut u8).unwrap();
        // let ide_tcb = ThreadControlBlock::new_witth_setup(ide_addr, true, &mut process);

        let block_manager = BlockManager::default();
        let input_buffer = Mutex::new(InputBuffer::new());

        // threads.scheduler.lock().push(Box::new(ide_tcb));
        threads.scheduler.lock().push(Box::new(my_tcb_one));
        threads.scheduler.lock().push(Box::new(my_tcb_two));
        threads.scheduler.lock().push(Box::new(my_tcb_three));

        println!("Mounting root filesystem...");
        let mut root = RootFileSystem::new();
        // for now, we just use TempFS for the root filesystem
        root.mount_root(TempFS::new())
            .expect("Couldn't mount root FS");

        crate::system::init_system(SystemState {
            threads,
            process,
            block_manager: RwLock::new(block_manager),
            root_filesystem: Mutex::new(root),
            input_buffer,
        });
        println!("initialized system");

        thread_system_start(page_manager, INIT);
    }
}
