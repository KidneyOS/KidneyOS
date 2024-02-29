use alloc::boxed::Box;
use arbitrary_int::u20;
use core::{arch::asm, mem::size_of, ops::Range};
use kidneyos_core::{
    mem::{
        phys::{kernel_data_start, kernel_end, kernel_start, main_stack_top, trampoline_heap_top},
        virt, OFFSET, PAGE_FRAME_SIZE,
    },
    paging::{PageDirectory, PageDirectoryEntry, PageTable, PageTableEntry, VirtualAddress},
    video_memory::{VIDEO_MEMORY_BASE, VIDEO_MEMORY_SIZE},
};

pub unsafe fn enable() {
    let page_directory = Box::leak(Box::<PageDirectory>::default());

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
            virt: virt::kernel_end(),
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

    for Region { phys, virt, write } in regions {
        for phys_addr in phys.clone().step_by(PAGE_FRAME_SIZE) {
            let virt_addr = phys_addr - phys.start + virt;
            let virt_addr = VirtualAddress::new_with_raw_value(virt_addr as u32);

            let page_directory_index: usize = virt_addr.page_directory_index().value().into();
            let page_table = if !page_directory[page_directory_index].present() {
                let page_table = Box::leak(Box::<PageTable>::default());
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
        in(reg) page_directory as *mut PageDirectory,
        out(reg) _,
    );
}
