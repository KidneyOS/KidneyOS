use super::FrameAllocator;
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

pub struct SubblockAllocatorSolution<F: FrameAllocator> {
    list_heads: [Option<&'static mut ListNode>; SUBBLOCK_TYPE_COUNT],
    frame_allocator: F,
}

impl<F: FrameAllocator> SubblockAllocatorSolution<F> {
    pub fn new(frame_allocator: F) -> Self {
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
            let num_frames = layout.size().max(layout.align()).div_ceil(PAGE_FRAME_SIZE);
            let new_frame = self.frame_allocator.alloc(num_frames)?;

            return Ok(new_frame.as_ptr());
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
    /// This function is unsafe because the caller must ensure the pointer belongs to the
    /// allocator
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
    pub fn get_frame_allocator(&mut self) -> &mut F {
        &mut self.frame_allocator
    }

    /// Uninitialize the subblock and frame allocator
    ///
    /// Returns true if no leaks, false if leaks
    /// TODO
    pub fn deinit(&mut self) -> bool {
        true
    }

    /// Check to see if the allocator is empty or not
    ///
    /// Returns true on empty, false if not
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        for i in 0..SUBBLOCK_TYPE_COUNT {
            if self.list_heads[i].is_some() {
                return false;
            }
        }

        true
    }

    /// Returns the length of the linked list corresponding to "subblock_size"
    ///
    /// Precondition: "subblock_size" must be a valid size in SUBBLOCK_SIZES
    ///
    /// Returns the length of the linked list on success, -1 on failure
    #[allow(dead_code)]
    pub fn length_of_lst(&self, subblock_size: usize) -> i32 {
        let mut index = SUBBLOCK_TYPE_COUNT;

        for (i, size) in SUBBLOCK_SIZES.into_iter().enumerate() {
            if subblock_size == size {
                index = i;
                break;
            }
        }

        if index == SUBBLOCK_TYPE_COUNT {
            return -1;
        }

        let mut length = 0;
        let mut head = &self.list_heads[index];
        while head.is_some() {
            length += 1;
            head = &head.as_ref().unwrap().next;
        }

        length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    use crate::mem::frame_allocator::placement_algorithms::NextFit;
    use crate::mem::frame_allocator::{CoreMapEntry, FrameAllocatorSolution};
    use std::{
        alloc::{Allocator, Global, Layout},
        error::Error,
    };

    #[test]
    fn test_subblock_allocator() -> Result<(), Box<dyn Error>> {
        const NUM_FRAMES: usize = 25;

        let core_map = [CoreMapEntry::DEFAULT; NUM_FRAMES];
        let layout = Layout::from_size_align(PAGE_FRAME_SIZE * NUM_FRAMES, PAGE_FRAME_SIZE)?;
        let region = Global.allocate(layout)?;

        let frame_allocator = FrameAllocatorSolution::<NextFit>::new(region, Box::new(core_map));

        let mut subblock_allocator = SubblockAllocatorSolution::new(frame_allocator);
        assert!(subblock_allocator.is_empty());

        // A request for 5 bytes should use a 16 byte subblock
        let layout_5_bytes = Layout::from_size_align(5, 2)?;
        let ptr_16_bytes = subblock_allocator.allocate(layout_5_bytes)?;

        assert!(!subblock_allocator.is_empty());
        assert_eq!(subblock_allocator.length_of_lst(16), 255);
        assert_eq!(subblock_allocator.get_frame_allocator().num_allocated(), 1);

        // A request for 97 bytes should use a 128 byte subblock
        let layout_97_bytes = Layout::from_size_align(97, 2)?;
        let ptr_128_bytes = subblock_allocator.allocate(layout_97_bytes)?;

        assert_eq!(subblock_allocator.length_of_lst(128), 31);
        assert_eq!(subblock_allocator.get_frame_allocator().num_allocated(), 2);

        // A request for 5000 bytes should use 2 frames
        let layout_5000_bytes = Layout::from_size_align(5000, 2)?;
        let frame_ptr = subblock_allocator.allocate(layout_5000_bytes)?;
        assert_eq!(subblock_allocator.get_frame_allocator().num_allocated(), 4);

        unsafe {
            subblock_allocator.deallocate(ptr_16_bytes, layout_5_bytes);
            subblock_allocator.deallocate(ptr_128_bytes, layout_97_bytes);
            subblock_allocator.deallocate(frame_ptr, layout_5000_bytes);
        }

        assert!(!subblock_allocator.is_empty());
        assert_eq!(subblock_allocator.length_of_lst(16), 256);
        assert_eq!(subblock_allocator.length_of_lst(128), 32);
        assert_eq!(subblock_allocator.get_frame_allocator().num_allocated(), 2);

        Ok(())
    }
}
