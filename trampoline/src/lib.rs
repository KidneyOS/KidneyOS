#![feature(asm_const)]
#![feature(naked_functions)]
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(not(test), no_main)]

mod multiboot2;

use arbitrary_int::u20;
use core::{arch::asm, mem::size_of, ops::Range};
use kidneyos_shared::{
    global_descriptor_table,
    mem::{
        phys::{
            kernel_data_start, kernel_end, kernel_start, main_stack_top, trampoline_data_start,
            trampoline_end, trampoline_heap_top, trampoline_start,
        },
        virt, OFFSET, PAGE_FRAME_SIZE,
    },
    paging::{PageDirectory, PageDirectoryEntry, PageTable, PageTableEntry, VirtualAddress},
    println,
    video_memory::{VIDEO_MEMORY_BASE, VIDEO_MEMORY_COLS, VIDEO_MEMORY_SIZE, VIDEO_MEMORY_WRITER},
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

    struct Region {
        phys: Range<usize>,
        virt: usize,
        write: bool,
    }
    let regions = [
        Region {
            phys: VIDEO_MEMORY_BASE..VIDEO_MEMORY_BASE + VIDEO_MEMORY_SIZE,
            virt: VIDEO_MEMORY_BASE,
            write: true,
        },
        Region {
            phys: trampoline_start()..trampoline_data_start(),
            virt: trampoline_start(),
            write: false,
        },
        Region {
            phys: trampoline_data_start()..trampoline_end(),
            virt: trampoline_data_start(),
            write: true,
        },
        Region {
            phys: kernel_start()..kernel_data_start(),
            virt: virt::kernel_start(),
            write: false,
        },
        Region {
            phys: kernel_data_start()..kernel_end(),
            virt: virt::kernel_data_start(),
            write: true,
        },
        Region {
            phys: kernel_end()..main_stack_top(),
            virt: kernel_end(),
            write: true,
        },
        Region {
            phys: kernel_end()..main_stack_top(),
            virt: virt::kernel_end(),
            write: true,
        },
        Region {
            phys: main_stack_top()..trampoline_heap_top(),
            virt: main_stack_top(),
            write: true,
        },
        // TODO: Use big pages so this doesn't take so long.
        Region {
            phys: trampoline_heap_top()..usize::MAX - OFFSET,
            virt: virt::trampoline_heap_top(),
            write: true,
        },
    ];
    assert!(regions
        .iter()
        .map(|Region { phys, .. }| phys.start)
        .all(|i| i % PAGE_FRAME_SIZE == 0));

    // TODO: Swap this out with a pool allocator.
    let mut next_addr = {
        let mut next_addr = main_stack_top() as *mut PageTable;
        assert!(next_addr as usize % PAGE_FRAME_SIZE == 0);
        move || {
            let res = next_addr;
            assert!((res as usize) < trampoline_heap_top());
            next_addr = next_addr.add(1);
            res
        }
    };

    let page_directory_addr = next_addr();
    let page_directory = &mut *(page_directory_addr as *mut PageDirectory);
    *page_directory = PageDirectory::default();

    for Region { phys, virt, write } in regions {
        for phys_addr in phys.clone().step_by(PAGE_FRAME_SIZE) {
            let virt_addr = phys_addr - phys.start + virt;
            let virt_addr = VirtualAddress::new_with_raw_value(virt_addr as u32);

            let page_directory_index: usize = virt_addr.page_directory_index().value().into();
            let page_table = if !page_directory[page_directory_index].present() {
                let page_table = &mut *next_addr();
                page_directory[page_directory_index] = PageDirectoryEntry::default()
                    .with_present(true)
                    .with_read_write(write)
                    .with_page_table_address(u20::new(
                        page_table as *mut PageTable as u32 / size_of::<PageTable>() as u32,
                    ));
                page_table
            } else {
                let page_table = &mut *((page_directory[page_directory_index]
                    .page_table_address()
                    .value() as usize
                    * size_of::<PageTable>())
                    as *mut PageTable);
                if write && !page_directory[page_directory_index].read_write() {
                    page_directory[page_directory_index] =
                        page_directory[page_directory_index].with_read_write(true);
                }
                page_table
            };

            let page_table_index: usize = virt_addr.page_table_index().value().into();
            page_table[page_table_index] = PageTableEntry::default()
                .with_present(true)
                .with_read_write(write)
                .with_page_frame_address(u20::new(phys_addr as u32 / PAGE_FRAME_SIZE as u32));
        }
    }

    asm!(
        "
        mov cr3, {0}
        mov {1}, cr0
        or {1}, 0x80010000
        mov cr0, {1}
        ",
        in(reg) page_directory as *mut PageDirectory as usize,
        out(reg) _,
    );

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
