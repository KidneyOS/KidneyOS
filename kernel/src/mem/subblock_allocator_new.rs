use core::{alloc::{AllocError, Layout}, mem::size_of};

use super::{FrameAllocatorWrapper};
use kidneyos_shared::mem::PAGE_FRAME_SIZE;
use kidneyos_shared::println;

const SUBBLOCK_TYPE_COUNT: usize = 8;
/// Allowed subblock sizes, which must be powers of 2.
const SUBBLOCK_SIZES: [usize; SUBBLOCK_TYPE_COUNT] = [16, 32, 64, 128, 256, 512, 1024, 2048];
const MAX_NUM_SUBBLOCKS: usize = 256;

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

struct ListNode {
    next: Option<&'static mut ListNode>,
}

pub struct SubblockAllocator {
    list_heads: [Option<&'static mut ListNode>; SUBBLOCK_TYPE_COUNT],
    frame_allocator: FrameAllocatorWrapper,
    mem_start: *mut u8
}

impl SubblockAllocator {
    pub fn new(mem_start: *mut u8, frame_allocator: FrameAllocatorWrapper) -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;

        SubblockAllocator {
            list_heads: [EMPTY; SUBBLOCK_TYPE_COUNT],
            mem_start,
            frame_allocator
        }
    }

    /// Allocate a fixed size block for the size requested by 'layout'
    ///
    pub fn allocate(&mut self, layout: Layout) -> Result<*mut u8, AllocError> {
        let subblock_size_index = get_best_subblock_size_idx(layout.size());

        if subblock_size_index == SUBBLOCK_TYPE_COUNT{
            println!("[SUBBLOCK ALLOCATOR]: Size requests larger than one frame not supported currently");
            return Err(AllocError);
        };

        match self.list_heads[subblock_size_index].take(){
            Some(node) => {
                // Set the new list head to be the next node from the current node
                //
                // Think of this as doing a linked_list.pop_from_head() operation
                println!("[SUBBLOCK ALLOCATOR]: List head exists, allocating subblock size: {}", SUBBLOCK_SIZES[subblock_size_index]);
                self.list_heads[subblock_size_index] = node.next.take();
                Ok(node as *mut ListNode as *mut u8)
            }
            None => {
                // If no block currently exists, we need to allocate a frame and divide the frame
                //
                // We first have to make sure the subblock size has enough room to hold a ListNode
                // This should always be the case, but we check regardless
                println!("[SUBBLOCK ALLOCATOR]: List head does not exist, requesting frame for subblock size: {}", SUBBLOCK_SIZES[subblock_size_index]);
                assert!(size_of::<ListNode>() <= SUBBLOCK_SIZES[subblock_size_index]);

                let new_frame = match self.frame_allocator.alloc(1) {
                    Err(AllocError) => return Err(AllocError),
                    Ok(v) => v
                };
                let start_of_frame_addr = new_frame.as_ptr() as *const u8 as usize;
                let num_subblocks = PAGE_FRAME_SIZE / SUBBLOCK_SIZES[subblock_size_index];

                // Begin to divide the frame into the required subblock sizes
                for i in 0..num_subblocks {
                    let start_of_subblock_addr = start_of_frame_addr + (SUBBLOCK_SIZES[subblock_size_index] * i);
                    let start_of_subblock_ptr = start_of_subblock_addr as *mut u8 as *mut ListNode;

                    let next = self.list_heads[subblock_size_index].take();
                    let new_node = ListNode{ next };

                    unsafe {
                        start_of_subblock_ptr.write(new_node);
                        self.list_heads[subblock_size_index] = Some(&mut *start_of_subblock_ptr)
                    }
                }

                // At this point, the head of the linked list should not be None
                assert!(!self.list_heads[subblock_size_index].is_none());

                // Return the head of the linked list
                let node = self.list_heads[subblock_size_index].take().unwrap();
                self.list_heads[subblock_size_index] = node.next.take();
                Ok(node as *mut ListNode as *mut u8)
            }
        }
    }

    /// Deallocate the region of memory pointed to by 'ptr' with size 'layout'
    ///
    /// When we deallocate, we simply write a new ListNode to the pointer we are given and
    /// make the new ListNode the head of the linked list
    ///
    /// This is very similar to linked_list.insert_head() operation
    pub fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        let subblock_size_index = get_best_subblock_size_idx(layout.size());

        assert!(size_of::<ListNode>() <= SUBBLOCK_SIZES[subblock_size_index]);

        let new_node = ListNode {
            next: self.list_heads[subblock_size_index].take(),
        };

        let new_node_ptr = ptr as *mut ListNode;
        unsafe {
            new_node_ptr.write(new_node);
            self.list_heads[subblock_size_index] = Some(&mut *new_node_ptr);
        }
    }

    /// Uninitialize the subblock and frame allocator
    /// TODO
    pub fn deinit(&mut self) -> bool{
        true
    }

    /// Return mutable reference to frame allocator
    pub fn get_frame_allocator(&mut self) -> &mut FrameAllocatorWrapper {
        &mut self.frame_allocator
    }
}
