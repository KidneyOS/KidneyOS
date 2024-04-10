use alloc::alloc::Global;
use kidneyos_shared::{
    mem::OFFSET,
    paging::{self, kernel_mapping_ranges},
};

pub type PageManager<A = Global> = paging::PageManager<A>;

pub unsafe fn enable() -> PageManager {
    let page_manager = PageManager::from_mapping_ranges_in(kernel_mapping_ranges(), Global, OFFSET);
    page_manager.load();
    page_manager
}
