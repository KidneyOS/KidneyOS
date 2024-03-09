use alloc::alloc::Global;
use core::mem::forget;
use kidneyos_shared::{
    mem::OFFSET,
    paging::{kernel_mapping_ranges, PageManager},
};

pub unsafe fn enable() {
    let page_manager = PageManager::from_mapping_ranges_in(kernel_mapping_ranges(), Global, OFFSET);
    page_manager.load();

    // TODO: Save this somewhere so it can be dropped when no longer in use.
    forget(page_manager);
}
