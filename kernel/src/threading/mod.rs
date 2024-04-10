mod context_switch;
pub mod scheduling;
mod thread_control_block;
mod thread_functions;

use crate::{
    paging::PageManager,
    sync::{intr_enable, intr_get_level, IntrLevel},
    user_program::{
        elf_loader::parse_elf,
        virtual_memory_area::{VmAreaStruct, VmFlags},
    },
};
use alloc::{alloc::Global, boxed::Box};
use core::ptr::copy_nonoverlapping;
use kidneyos_shared::{
    mem::{OFFSET, PAGE_FRAME_SIZE},
    paging::kernel_mapping_ranges,
    println,
};
use scheduling::{initialize_scheduler, scheduler_yield, SCHEDULER};
use thread_control_block::{ThreadControlBlock, Tid};

static mut RUNNING_THREAD: Option<Box<ThreadControlBlock>> = None;

/// To be called before any other thread functions.
/// To be called with interrupts disabled.
static mut THREAD_SYSTEM_INITIALIZED: bool = false;
pub fn thread_system_initialization() {
    println!("Initializing Thread System...");

    assert!(intr_get_level() == IntrLevel::IntrOff);

    // Initialize the TID lock.

    // Initialize the scheduler.
    initialize_scheduler();

    // Create Idle thread.

    // SAFETY: Interrupts must be disabled.
    unsafe {
        THREAD_SYSTEM_INITIALIZED = true;
    }
    println!("Finished Thread System initialization. Ready to start threading.");
}

/// Enables preemptive scheduling.
/// Thread system must have been previously enabled.
pub fn thread_system_start(kernel_page_manager: PageManager, init_elf: &[u8]) -> ! {
    assert!(intr_get_level() == IntrLevel::IntrOff);
    assert!(
        unsafe { THREAD_SYSTEM_INITIALIZED },
        "Cannot start threading without initializing the threading system."
    );

    // We must 'turn the kernel thread into a thread'.
    // This amounts to just making a TCB that will be in control of the kernel stack and will
    // never exit.
    // This thread also does not need to enter the `run_thread` function.
    // SAFETY: The kernel thread's stack will be set up by the context switch following.
    let tcb_kernel = unsafe { ThreadControlBlock::create_kernel_thread(kernel_page_manager) };

    let (entrypoint, vm_areas) = parse_elf(init_elf).expect("Init ELF was malformed.");

    let mut init_page_manager =
        PageManager::from_mapping_ranges_in(kernel_mapping_ranges(), Global, OFFSET);
    for VmAreaStruct {
        vm_start,
        vm_end,
        offset,
        flags:
            VmFlags {
                read: _,
                write: _,
                execute: _,
                shared: _,
                private: _,
            },
    } in vm_areas
    {
        let len = vm_end - vm_start;
        let frames = len.div_ceil(PAGE_FRAME_SIZE);

        let phys_addr = unsafe {
            crate::KERNEL_ALLOCATOR
                .frame_alloc(frames)
                .expect("no more frames...")
                .cast::<u8>()
                .as_ptr()
                .sub(OFFSET)
        };

        // SAFETY: this page manager is not yet activated.
        unsafe {
            init_page_manager.map_range(
                phys_addr as usize,
                vm_start,
                frames * PAGE_FRAME_SIZE,
                true, // TODO: Drop write privilege after we copy if necessary
                true,
            )
        };

        // TODO: Probably not how we should do this...
        unsafe { init_page_manager.load() };

        unsafe { copy_nonoverlapping(&init_elf[offset] as *const u8, vm_start as *mut u8, len) };
        // TODO: Zero bytes from len to the range mapped above.
    }

    let init_tcb = ThreadControlBlock::create(entrypoint, init_page_manager);

    // SAFETY: Interrupts must be disabled.
    unsafe {
        RUNNING_THREAD = Some(Box::new(tcb_kernel));
        SCHEDULER
            .as_mut()
            .expect("No Scheduler set up!")
            .push(Box::new(init_tcb));
    }

    // Enable preemptive scheduling.
    intr_enable();

    // Eventually, the scheduler may run the kernel thread again.
    // We may later replace this with code to clean up the kernel resources (`thread_exit` would not work).
    // For now we will just yield continually.
    loop {
        scheduler_yield();
    }

    // This function never returns.
}
