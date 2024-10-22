use alloc::vec::Vec;
use core::alloc::Allocator;
use core::slice::from_raw_parts;
use kidneyos_shared::mem::OFFSET;
use kidneyos_shared::paging::PageManager;

pub fn check_and_copy_user_memory<A: Allocator>(
    pointer: usize,
    count: usize,
    page_manager: &PageManager<A>,
) -> Option<Vec<u8>> {
    let range_end = pointer + count;

    // Trying to read from kernel memory.
    if range_end >= OFFSET {
        return None;
    }

    if !page_manager.is_range_mapped(pointer, count) {
        return None;
    }

    let bytes = unsafe { from_raw_parts(pointer as *const u8, count) };

    // We sometimes want to transfer information from one thread to another.
    // To avoid having to map this memory across threads, we copy it to kernel memory first.
    Some(bytes.to_vec())
}
