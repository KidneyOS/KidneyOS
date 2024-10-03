use alloc::alloc::Global;
use kidneyos_shared::{
    mem::OFFSET,
    paging::{self, kernel_mapping_ranges},
};

pub type PageManager<A = Global> = paging::PageManager<A>;

pub trait PageManagerDefault {
    fn default() -> Self;
}

impl PageManagerDefault for PageManager<Global> {
    fn default() -> Self {
        PageManager::from_mapping_ranges_in(kernel_mapping_ranges(), Global, OFFSET)
    }
}

pub unsafe fn enable() -> PageManager {
    let page_manager = PageManager::default();
    page_manager.load();
    page_manager
}

pub fn is_userspace_readable(ptr: *const u8) -> bool {
    if (ptr as isize) <= 0 {
        // null or kernel-space address
        return false;
    }
    // TODO: check page table
    true
}

pub fn is_userspace_writeable(ptr: *const u8) -> bool {
    if (ptr as isize) <= 0 {
        // null or kernel-space address
        return false;
    }
    // TODO: check page table
    true
}
