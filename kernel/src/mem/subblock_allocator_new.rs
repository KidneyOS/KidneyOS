use core::{
    alloc::{AllocError, Allocator, Layout},
    ptr,
    ptr::NonNull,
    slice,
};

use alloc::boxed::Box;

use crate::mem::frame_allocator::{CoreMapEntry, FrameAllocatorSolution};
use crate::mem::FrameAllocator;
use kidneyos_shared::mem::PAGE_FRAME_SIZE;

const SUBBLOCK_TYPE_COUNT: usize = 8;
/// Allowed subblock sizes, which must be powers of 2.
const SUBBLOCK_SIZES: [usize; SUBBLOCK_TYPE_COUNT] = [16, 32, 64, 128, 256, 512, 1024, 2048];
const MAX_NUM_SUBBLOCKS: usize = 256;

/// A linked list node representing a frame.
#[derive(Default)]
struct ListNode {
    /// Next frame with the same subblock size.
    next: Option<Box<ListNode>>,
    /// Frame number of the frame represented by this node.
    frame_number: Option<usize>,
}

/// A linked list of nodes representing frames.
///
/// The head is stored on the stack to enable "bootstrapping".
#[derive(Default)]
struct List {
    head: ListNode,
}

/// Returns the smallest subblock size that fits the given number of bytes
/// in terms of its index in `SUBBLOCK_SIZES`. Returns SUBBLOCK_TYPE_COUNT
/// if the requested size is larger than the largest subblock size.
fn get_best_subblock_size_idx(num_bytes: usize) -> usize {
    if num_bytes > PAGE_FRAME_SIZE {
        panic!("Requested memory size larger than page frame size");
    }
    for (index, size) in SUBBLOCK_SIZES.into_iter().enumerate() {
        if num_bytes <= size {
            return index;
        }
    }
    SUBBLOCK_TYPE_COUNT
}

pub struct SubblockAllocator {
    lists: [List; SUBBLOCK_TYPE_COUNT],
    frame_allocator: FrameAllocatorSolution,
    mem_start: NonNull<u8>,
}

impl SubblockAllocator {
    pub fn new(mem_start: NonNull<u8>, frame_allocator: FrameAllocatorSolution) -> Self {
        let stub = List {
            head: ListNode {
                next: None,
                frame_number: None,
            },
        };
        SubblockAllocator {
            lists: Default::default(),
            frame_allocator,
            mem_start,
        }
    }

    fn allocate(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // TODO: Ignoring alignment for now
        // TODO: Technically size may be 0, but we also ignore that for now
        let subblock_size_index = get_best_subblock_size_idx(layout.size());
        if subblock_size_index == SUBBLOCK_TYPE_COUNT {
            // Requested size larger than largest subblock size, fall back to frame allocator
            let frames_to_allocate = layout.size().div_ceil(PAGE_FRAME_SIZE);
            let region = self.frame_allocator.alloc(frames_to_allocate)?;
            let start_addr = region.as_ptr() as *mut u8;
            let slice_ptr = NonNull::slice_from_raw_parts(
                NonNull::new(start_addr).expect("start_addr shouldn't be null"),
                layout.size(),
            );
            return Ok(slice_ptr);
        }
        // TODO: If there is space, allocate from the list

        // No free space in existing frames, need a new frame
        let new_frame = self.frame_allocator.alloc(1)?;
        // TODO: Since we need to traverse the whole list anyway, we will be able
        // to save a reference to the last node without additional overhead.
        // For now we just take head as last_node.
        let last_node = &mut self.lists[subblock_size_index].head;
        last_node.frame_number = Some(
            new_frame.as_ptr() as *const u8 as usize
                - self.mem_start.as_ptr() as usize / PAGE_FRAME_SIZE,
        );
    }

    unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        todo!()
    }
}
