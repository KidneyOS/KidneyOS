// https://wiki.osdev.org/Paging
// https://wiki.osdev.org/Setting_Up_Paging

// Avoids lots of warnings about casting usize to u32 which cannot result in
// truncation on a 32-bit platform, which is all we support. It would be nice if
// you could tell clippy that you were only dealing with 32-bit usizes...
#![allow(clippy::cast_possible_truncation)]

use alloc::boxed::Box;
use arbitrary_int::{u10, u12, u20};
use bitbybit::bitfield;
use core::{
    arch::asm,
    mem::size_of,
    ops::{Deref, DerefMut, Range},
    ptr::null_mut,
};
use kidneyos::{
    mem::{KERNEL_DATA_OFFSET, KERNEL_MAIN_STACK_TOP, KERNEL_MAX, KERNEL_OFFSET, PAGE_FRAME_SIZE},
    video_memory::{VIDEO_MEMORY_BASE, VIDEO_MEMORY_SIZE},
};

const PAGE_DIRECTORY_LEN: usize = PAGE_FRAME_SIZE / size_of::<PageDirectoryEntry>();

#[repr(align(4096))]
pub struct PageDirectory([PageDirectoryEntry; PAGE_DIRECTORY_LEN]);

impl Default for PageDirectory {
    fn default() -> Self {
        Self([PageDirectoryEntry::default(); PAGE_DIRECTORY_LEN])
    }
}

impl Deref for PageDirectory {
    type Target = [PageDirectoryEntry; PAGE_DIRECTORY_LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PageDirectory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[bitfield(u32, default = 0)]
pub struct PageDirectoryEntry {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    read_write: bool,
    #[bit(2, rw)]
    user_supervisor: bool,
    #[bit(3, rw)]
    write_through: bool,
    #[bit(4, rw)]
    cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(7, rw)]
    page_size: bool,
    #[bits(12..=31, rw)]
    page_table_address: u20,
}

const PAGE_TABLE_LEN: usize = PAGE_FRAME_SIZE / size_of::<PageTableEntry>();

#[repr(align(4096))]
pub struct PageTable([PageTableEntry; PAGE_TABLE_LEN]);

impl Default for PageTable {
    fn default() -> Self {
        Self([PageTableEntry::default(); PAGE_TABLE_LEN])
    }
}

impl Deref for PageTable {
    type Target = [PageTableEntry; PAGE_TABLE_LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[bitfield(u32, default = 0)]
pub struct PageTableEntry {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    read_write: bool,
    #[bit(2, rw)]
    user_supervisor: bool,
    #[bit(3, rw)]
    write_through: bool,
    #[bit(4, rw)]
    cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(6, rw)]
    dirty: bool,
    #[bit(7, rw)]
    page_attribute_table: bool,
    #[bit(8, rw)]
    global: bool,
    #[bits(12..=31, rw)]
    page_frame_address: u20,
}

#[bitfield(u32)]
struct VirtualAddress {
    #[bits(22..=31, r)]
    page_directory_index: u10,
    #[bits(12..=21, r)]
    page_table_index: u10,
    #[bits(0..=11, r)]
    offset: u12,
}

static mut TOP_LEVEL_PAGE_DIRECTORY: *mut PageDirectory = null_mut();

pub unsafe fn enable(kernel_memory_range: Range<usize>) {
    TOP_LEVEL_PAGE_DIRECTORY = Box::leak(Box::<PageDirectory>::default());
    let page_directory = &mut *TOP_LEVEL_PAGE_DIRECTORY;

    struct PageRegion {
        address_range: Range<usize>,
        read_write: bool,
    }
    let regions = [
        PageRegion {
            address_range: VIDEO_MEMORY_BASE..VIDEO_MEMORY_BASE + VIDEO_MEMORY_SIZE,
            read_write: true,
        },
        PageRegion {
            address_range: KERNEL_OFFSET..KERNEL_DATA_OFFSET,
            read_write: false,
        },
        PageRegion {
            address_range: KERNEL_DATA_OFFSET..KERNEL_MAX,
            read_write: true,
        },
        PageRegion {
            address_range: KERNEL_MAX..KERNEL_MAIN_STACK_TOP,
            read_write: true,
        },
        PageRegion {
            address_range: kernel_memory_range,
            read_write: true,
        },
    ];
    assert!(regions
        .iter()
        .map(|r| r.address_range.start)
        .all(|i| i % PAGE_FRAME_SIZE == 0));

    for region in &regions {
        for physical_addr in region.address_range.clone().step_by(PAGE_FRAME_SIZE) {
            let virtual_addr = VirtualAddress::new_with_raw_value(physical_addr as u32);

            let page_directory_index: usize = virtual_addr.page_directory_index().value().into();
            let page_table = if !page_directory[page_directory_index].present() {
                let page_table = Box::leak(Box::<PageTable>::default());
                page_directory[page_directory_index] = PageDirectoryEntry::default()
                    .with_present(true)
                    .with_read_write(region.read_write)
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
                if region.read_write && !page_directory[page_directory_index].read_write() {
                    page_directory[page_directory_index] =
                        page_directory[page_directory_index].with_read_write(true);
                }
                page_table
            };

            let page_table_index: usize = virtual_addr.page_table_index().value().into();
            page_table[page_table_index] = PageTableEntry::default()
                .with_present(true)
                .with_read_write(region.read_write)
                .with_page_frame_address(u20::new(physical_addr as u32 / PAGE_FRAME_SIZE as u32));
        }
    }

    asm!(
        "
        mov cr3, {0}
        mov {1}, cr0
        or {1}, 0x80010000
        mov cr0, {1}
        ",
        in(reg) TOP_LEVEL_PAGE_DIRECTORY,
        out(reg) _,
    );
}
