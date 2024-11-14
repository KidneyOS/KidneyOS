use super::{FrameAllocator, FrameAllocatorSolution};
use core::ptr::NonNull;
use core::{
    alloc::{AllocError, Layout},
    mem::size_of,
};
use kidneyos_shared::mem::PAGE_FRAME_SIZE;

const SUBBLOCK_TYPE_COUNT: usize = 8;
/// Allowed subblock sizes, which must be powers of 2.
const SUBBLOCK_SIZES: [usize; SUBBLOCK_TYPE_COUNT] = [16, 32, 64, 128, 256, 512, 1024, 2048];

/// Returns the smallest subblock size that fits the given number of bytes
/// in terms of its index in `SUBBLOCK_SIZES`. Returns SUBBLOCK_TYPE_COUNT
/// if the requested size is larger than the largest subblock size.
fn get_best_subblock_size_idx(layout: Layout) -> usize {
    let num_bytes = layout.size().max(layout.align());

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

pub struct SubblockAllocatorSolution {
    list_heads: [Option<&'static mut ListNode>; SUBBLOCK_TYPE_COUNT],
    frame_allocator: FrameAllocatorSolution,
}

impl SubblockAllocatorSolution {
    pub fn new(frame_allocator: FrameAllocatorSolution) -> SubblockAllocatorSolution {
        const EMPTY: Option<&'static mut ListNode> = None;

        SubblockAllocatorSolution {
            list_heads: [EMPTY; SUBBLOCK_TYPE_COUNT],
            frame_allocator,
        }
    }

    /// Allocate a fixed size block for the size requested by 'layout'
    ///
    /// Allocations always come from the head of each LinkedList - this is very similar to
    /// LinkedList.pop() operation
    ///
    /// If the LinkedList is empty, a frame is allocated and dividing into chunks equal to
    /// the size of the subblock size corresponding to that LinkedList
    ///
    /// If the allocation size is larger than the largest subblock size (2048 bytes), a frame(s)
    /// is allocated instead of dividing into subblocks
    pub fn allocate(&mut self, layout: Layout) -> Result<*mut u8, AllocError> {
        let subblock_size_index = get_best_subblock_size_idx(layout);

        if subblock_size_index == SUBBLOCK_TYPE_COUNT {
            let num_frames = layout
                .size()
                .max(layout.align())
                .next_multiple_of(PAGE_FRAME_SIZE);

            if !self.frame_allocator.has_room(num_frames) {
                return Err(AllocError);
            }

            let new_frame = self.frame_allocator.alloc(num_frames)?;

            return Ok(new_frame.as_ptr() as *mut u8);
        };

        match self.list_heads[subblock_size_index].take() {
            Some(node) => {
                // Set the new list head to be the next node from the current node
                //
                // Think of this as doing a linked_list.pop_from_head() operation
                self.list_heads[subblock_size_index] = node.next.take();
                Ok(node as *mut ListNode as *mut u8)
            }
            None => {
                // If no block currently exists, we need to allocate a frame and divide the frame
                //
                // We first have to make sure the subblock size has enough room to hold a ListNode
                // This should always be the case, but we check regardless
                assert!(size_of::<ListNode>() <= SUBBLOCK_SIZES[subblock_size_index]);

                if !self.frame_allocator.has_room(1) {
                    return Err(AllocError);
                }

                let new_frame = self.frame_allocator.alloc(1)?;

                let start_of_frame = new_frame.as_ptr() as *const u8;
                let num_subblocks = PAGE_FRAME_SIZE / SUBBLOCK_SIZES[subblock_size_index];

                // Begin to divide the frame into the required subblock sizes
                for i in 0..num_subblocks {
                    let next = self.list_heads[subblock_size_index].take();
                    let new_node = ListNode { next };

                    unsafe {
                        let start_of_subblock_ptr = start_of_frame
                            .add(SUBBLOCK_SIZES[subblock_size_index] * i)
                            as *mut ListNode;
                        start_of_subblock_ptr.write(new_node);
                        self.list_heads[subblock_size_index] = Some(&mut *start_of_subblock_ptr)
                    }
                }

                // At this point, the head of the linked list should not be None
                assert!(self.list_heads[subblock_size_index].is_some());

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
    ///
    /// TODO: Reclaim fully freed frames from free subblocks
    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        let subblock_size_index = get_best_subblock_size_idx(layout);

        if subblock_size_index == SUBBLOCK_TYPE_COUNT {
            self.frame_allocator.dealloc(NonNull::new(ptr).unwrap());
        } else {
            assert!(size_of::<ListNode>() <= SUBBLOCK_SIZES[subblock_size_index]);

            let new_node = ListNode {
                next: self.list_heads[subblock_size_index].take(),
            };

            let new_node_ptr = ptr as *mut ListNode;
            new_node_ptr.write(new_node);
            self.list_heads[subblock_size_index] = Some(&mut *new_node_ptr);
        }
    }

    /// Return a mutable reference to underlying frame allocator
    ///
    /// This function should be used for memory allocations that do not go through the kernel
    /// allocator and instead requests directly from the frame allocator
    pub fn get_frame_allocator(&mut self) -> &mut FrameAllocatorSolution {
        &mut self.frame_allocator
    }

    /// Uninitialize the subblock and frame allocator
    ///
    /// Returns true if no leaks, false if leaks
    /// TODO
    pub fn deinit(&mut self) -> bool {
        true
    }
}
