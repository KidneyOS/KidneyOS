use alloc::alloc::Global;
use kidneyos_shared::{
    mem::OFFSET,
    paging::{kernel_mapping_ranges, PageManager},
};

pub unsafe fn enable() -> PageManager<Global> {
    let page_manager = PageManager::from_mapping_ranges_in(kernel_mapping_ranges(), Global, OFFSET);
    page_manager.load();
    page_manager
}
