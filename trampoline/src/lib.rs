#![feature(asm_const)]
#![feature(naked_functions)]
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(not(test), no_main)]

mod multiboot2;

use core::{arch::asm, ptr::NonNull};
use kidneyos_shared::{
    global_descriptor_table,
    mem::{
        phys::{
            kernel_end, main_stack_top, trampoline_data_start, trampoline_end, trampoline_heap_top,
            trampoline_start,
        },
        pool_allocator::PoolAllocator,
        OFFSET, PAGE_FRAME_SIZE,
    },
    paging::{self, kernel_mapping_ranges, PageManager},
    println,
    video_memory::{VIDEO_MEMORY_COLS, VIDEO_MEMORY_WRITER},
};
use multiboot2::{
    info::{Info, InfoTag},
    EXPECTED_MAGIC,
};

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(args: &core::panic::PanicInfo) -> ! {
    kidneyos_shared::eprintln!("{}", args);
    loop {}
}

#[cfg_attr(not(test), no_mangle)]
#[naked]
unsafe extern "C" fn _start() {
    core::arch::asm!(
        "
        lea esp, kernel_end
        sub esp, {stack_size}
        push ebx
        push eax
        call {}
        ",
        sym trampoline,
        stack_size = const kidneyos_shared::mem::OFFSET - kidneyos_shared::mem::MAIN_STACK_SIZE,
        options(noreturn),
    )
}

#[allow(dead_code)]
unsafe extern "C" fn trampoline(magic: usize, multiboot2_info: *mut Info) {
    assert!(
        magic == EXPECTED_MAGIC,
        "invalid magic, expected {EXPECTED_MAGIC:#X}, got {magic:#X}"
    );

    let mem_upper = (*multiboot2_info)
        .iter()
        .find_map(|tag| match tag {
            InfoTag::BasicMemoryInfo(t) => Some(t.mem_upper),
            _ => None,
        })
        .expect("Didn't find memory info!");

    println!("Setting up GDTR");
    global_descriptor_table::load();
    println!("GDTR set up!");

    println!("Enabling paging");

    let pool_region = NonNull::slice_from_raw_parts(
        NonNull::new(main_stack_top() as *mut u8).expect("main_stack_top shouldn't be null"),
        trampoline_heap_top() - main_stack_top(),
    );
    let alloc = PoolAllocator::<PAGE_FRAME_SIZE>::new(pool_region);
    let mut page_manager = PageManager::from_mapping_ranges_in(kernel_mapping_ranges(), alloc, 0);

    // Trampoline mappings.
    page_manager.id_map_range(
        trampoline_start(),
        trampoline_data_start() - trampoline_start(),
        false,
    );
    page_manager.id_map_range(
        trampoline_data_start(),
        trampoline_end() - trampoline_data_start(),
        true,
    );
    page_manager.id_map_range(kernel_end(), main_stack_top() - kernel_end(), true);
    page_manager.id_map_range(
        main_stack_top(),
        trampoline_heap_top() - main_stack_top(),
        true,
    );

    page_manager.load();
    paging::enable();

    println!("Paging enabled!");

    println!("Starting kernel...");

    extern "C" {
        fn main(mem_upper: usize, video_memory_skip_lines: usize) -> !;
    }

    asm!(
        "
        add esp, {offset} // make stack a kernel virtual address
        push {}
        push {}
        call {}
        ",
        in(reg) VIDEO_MEMORY_WRITER.cursor.div_ceil(VIDEO_MEMORY_COLS),
        in(reg) mem_upper as usize,
        sym main,
        offset = const OFFSET,
        options(noreturn)
    );
}
