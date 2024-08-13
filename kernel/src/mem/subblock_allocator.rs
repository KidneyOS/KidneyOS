use core::{
    alloc::{AllocError, Layout},
    ptr,
    ptr::NonNull,
    slice,
};
use std::mem::size_of;
use bitvec::{bits};
use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::view::BitViewSized;
use kidneyos_shared::mem::PAGE_FRAME_SIZE;
use crate::mem::frame_allocator::{FrameAllocatorSolution, CoreMapEntry};
use crate::mem::{FrameAllocator};

struct ListNode<> {
    next: Option<& 'static mut ListNode>,
    free_subblocks: Option<usize>,
    this_frame: Option<usize>,
    frames: Option<& 'static BitSlice<usize, Lsb0>>,
}

impl ListNode {
    const fn new() -> Self {
        ListNode { next: None, free_subblocks: None, this_frame: None, frames: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> *mut u8 {
        (self.start_addr() + size_of::<ListNode>()) as *mut u8
    }

    fn use_subblock(&mut self) {
        if let Some(curr_subblocks) = self.free_subblocks {
            self.free_subblocks = Some(curr_subblocks + 1);
        } else {
            self.free_subblocks = Some(1);
        }
    }

    fn free_subblock(&mut self) {
        // Preconditions: self.free_subblocks != None and self.free_subblocks > 0
        if let Some(curr_subblocks) = self.free_subblocks {
            self.free_subblocks = Some(curr_subblocks - 1);
        }
    }

    fn is_using_frame(&self, index: usize) -> bool {
        if let Some(frames) = self.frames {
            if index < frames.len() {
                return frames.get(index).as_deref() == Some(&true);
            }
        }
        false
    }

    fn set_frame(&mut self, index: usize) {
        // Helper function for set_frame_used and set_frame_unused
        if let Some(frames) = self.frames {
            if index >= frames.len() {
                // Frame index out of range (need more bits)
                let new_length = frames.len() * 2;
                let mut temp_bit_vec = frames.repeat(2);
                temp_bit_vec.shift_left(frames.len());
                temp_bit_vec.shift_right(frames.len());
                let mut new_bit_slice = &temp_bit_vec[..];
                assert_eq!(new_bit_slice.len(), new_length); // Sanity check
                self.frames = Option::from(new_bit_slice);
            }
        } else {
            // First frame allocated
            let bit_slice = bits!(0; 8);
            self.frames = Some(bit_slice);
        }
    }

    fn set_frame_used(&mut self, index: usize) {
        self.set_frame(index);
        if let Some(mut frames) = self.frames {
            frames.set(index, true);
            self.frames = Some(frames); // Potentially unnecessary
        }
    }

    fn set_frame_unused(&mut self, index: usize) {
        self.set_frame(index);
        if let Some(mut frames) = self.frames {
            frames.set(index, false);
            self.frames = Some(frames); // Potentially unnecessary
        }
    }

    fn check_frame_used(&self, index: usize) -> bool {
        if let Some(frames) = self.frames {
            if let Some(bit_val) = frames.get(index).as_deref() {
                return *bit_val;
            }
        }
        return false;
    }
}


// Potential subblock sizes. Each size must be a power of 2, and the smallest
// allowable size is 8 bytes
const SUBBLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

pub struct SubblockAllocator {
    list_heads: [Option<&'static mut ListNode>; SUBBLOCK_SIZES.len()],
    fallback_allocator: FrameAllocatorSolution,
    mem_start: NonNull<u8>,
}

fn get_nearest_subblock_size(num_bytes: usize) -> usize {
    for i in SUBBLOCK_SIZES {
        if num_bytes <= *i {
            return *i;
        }
    }
    PAGE_FRAME_SIZE
}


fn get_subblock_size_index(num_bytes: usize) -> usize {
    // Precondition: num_bytes is an element of SUBBLOCK_SIZES
    //
    // Returns 999 on error (should never happen)
    let mut index = 0;
    for i in SUBBLOCK_SIZES {
        if num_bytes == *i {
            return index;
        }
        index += 1;
    }
    999
}


impl SubblockAllocator {
    /// SubblockAllocator has the giant chunk of memory referred to in region.
    /// The start of the region, a subblock of smallest possible size, contains
    /// the size of the subblock storing the heads of each subblock type. The
    /// subblock immediately following this first subblock contains all the
    /// heads, in order of magnitude. For instance, the head pointing to the
    /// next free subblock of the third-smallest subblock type is stored at
    ///
    ///    <region start address> + <smallest block size> + <size of two ListNode objects>
    ///
    /// The first <size of ListNode> bytes of each allocated subblock
    /// contain a pointer to the previously allocated subblock of the same type
    ///
    /// # Safety
    ///
    /// region must be valid for the lifetime of this SubblockAllocator.
    ///
    pub const fn new(mem_start: NonNull<u8>, core_map: Box<[CoreMapEntry]>, total_number_of_frames: usize) -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        SubblockAllocator {
            list_heads: [EMPTY; SUBBLOCK_SIZES.len()],
            fallback_allocator: FrameAllocatorSolution::new_in(mem_start, core_map, total_number_of_frames),
            mem_start,
        }
    }

    unsafe fn initialize_subblock(&mut self, subblock_region: NonNull<[u8]>, bytes_to_allocate: usize) {
        let subblock_size_index = get_subblock_size_index(bytes_to_allocate);
        let bitmap_size = (PAGE_FRAME_SIZE / bytes_to_allocate).div_ceil(8);
        let region_ptr = subblock_region.as_mut_ptr();
        let frame_num = ((region_ptr as usize) - (self.mem_start.as_ptr() as usize)) / PAGE_FRAME_SIZE;

        let mut use_head = false;
        if let Some(ref mut head_node) = self.list_heads[subblock_size_index].as_mut() {
            ptr::write(region_ptr.cast::<ListNode>(), ListNode { next: head_node.next.take(), free_subblocks: None, this_frame: None, frames: None });
            head_node.next = Some(&mut *region_ptr.cast::<ListNode>());
            if let Some(next_node) = head_node.next {
                next_node.this_frame = Some(frame_num);
            }
        } else {
            ptr::write(region_ptr.cast::<ListNode>(), ListNode { next: None, free_subblocks: None, this_frame: None, frames: None });
            self.list_heads[subblock_size_index] = Some(&mut *region_ptr.cast::<ListNode>());
            if let Some(head_node) = self.list_heads[subblock_size_index] {
                head_node.this_frame = Some(frame_num);
            }
            use_head = true;
        }

        ptr::write_bytes(region_ptr.cast::<u8>().add(size_of::<ListNode>()), 0, bitmap_size);
        let bitmap_slice = slice::from_raw_parts_mut(region_ptr.cast::<u8>().add(size_of::<ListNode>()), bitmap_size);
        let mut i = 0;
        let num_used_subblocks = (bitmap_size + size_of::<ListNode>()).div_ceil(bytes_to_allocate);
        loop {
            let byte_index = i / 8;
            let bit_index = i % 8;
            if i < num_used_subblocks {
                bitmap_slice[byte_index] |= 1 << bit_index;
                if let Some(new_node) = self.list_heads[subblock_size_index] {
                    if use_head {
                        new_node.use_subblock();
                    } else if let Some(ref mut next_node) = new_node.next.as_mut() {
                        next_node.use_subblock();
                    }
                }
            } else {
                break;
            }
            i += 1;
        }

        // The following is always performed on the head of the linked list
        if let Some(list_head) = self.list_heads[subblock_size_index] {
            list_head.set_frame_used(frame_num);
        }
    }

    fn allocate(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let bytes_to_allocate = get_nearest_subblock_size(layout.size());

        // More memory was requested than the largest subblock size
        if bytes_to_allocate == PAGE_FRAME_SIZE {
            let frames_to_allocate = layout.size().div_ceil(PAGE_FRAME_SIZE);
            let region_ref = self.fallback_allocator.alloc(frames_to_allocate);
            return match region_ref {
                Ok(region) => {
                    let start_addr = region.as_ptr() as *mut u8;
                    let slice_ptr = NonNull::slice_from_raw_parts(
                        NonNull::new(start_addr).expect("start_addr shouldn't be null"),
                        layout.size(),
                    );
                    Ok(slice_ptr)
                }
                Err(error) => {
                    Err(error)
                }
            }
        }

        // Less memory was requested than the largest subblock size
        let subblock_size_index = get_subblock_size_index(bytes_to_allocate);
        let bitmap_size = (PAGE_FRAME_SIZE / bytes_to_allocate).div_ceil(8); // num bytes taken by bitmap

        if self.list_heads[subblock_size_index].is_none() {   // Memory has never been allocated at this size
            let subblock_region_ref = self.fallback_allocator.alloc(1);
            match subblock_region_ref {
                Ok(subblock_region) => {
                    unsafe { self.initialize_subblock(subblock_region, bytes_to_allocate); }
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }

        let mut list_node_ref: Option<&mut ListNode>;
        {
            let list_heads_ref = &mut self.list_heads;
            if let Some(node_ref) = list_heads_ref[subblock_size_index].as_mut() {
                list_node_ref = Some(node_ref);
            } else {
                return Err(AllocError);
            }
        }

        while let Some(node) = list_node_ref {
            let mut free_block_idx = 0;
            unsafe {
                let bitmap_start_addr = node.end_addr();
                let bitmap_slice = slice::from_raw_parts_mut(bitmap_start_addr, bitmap_size);
                let mut found_free_block = false;

                for byte_index in 0..bitmap_size {
                    for bit_index in 0..8 {
                        if !((bitmap_slice[byte_index] & (1 << bit_index)) != 0) {
                            found_free_block = true;
                            free_block_idx = byte_index + bit_index;
                            bitmap_slice[byte_index] = bitmap_slice[byte_index] | 1 << bit_index;
                            break;
                        }
                    }
                    if found_free_block {
                        let start_addr = (node.start_addr() + (bytes_to_allocate * free_block_idx)) as *mut u8;
                        let slice_ptr = NonNull::slice_from_raw_parts(
                            NonNull::new(start_addr).expect("start_addr shouldn't be null"),
                            layout.size(),
                        );
                        node.use_subblock();
                        return Ok(slice_ptr);
                    }
                }
            }
            if let Some(next_node) = node.next {
                list_node_ref = Some(next_node);
            } else {
                break;
            }
        }

        // No room in any subblock of correct size. Allocate new one
        let subblock_region = self.fallback_allocator.alloc(1);
        if subblock_region.is_err() { return Err(AllocError); }
        unsafe {
            let subblock_region = subblock_region.unwrap();
            self.initialize_subblock(subblock_region, bytes_to_allocate);

            if let Some(ref mut head_ref) = self.list_heads[subblock_size_index].as_mut() {
                if let Some(ref mut new_node) = head_ref.next {
                    // Calculate start address (will always be first subblock after list node + bitmap)
                    let num_used_subblocks = (bitmap_size + size_of::<ListNode>()).div_ceil(bytes_to_allocate);
                    let start_addr = (new_node.start_addr() + (bytes_to_allocate * num_used_subblocks)) as *mut u8;

                    // Set bitmap bit
                    let bitmap_start_addr = new_node.end_addr();
                    let bitmap_slice = slice::from_raw_parts_mut(bitmap_start_addr, bitmap_size);
                    let byte_index = num_used_subblocks / 8;
                    let bit_index = num_used_subblocks % 8;
                    bitmap_slice[byte_index] = bitmap_slice[byte_index] | 1 << bit_index;

                    // Return pointer
                    let slice_ptr = NonNull::slice_from_raw_parts(
                        NonNull::new(start_addr).expect("start_addr shouldn't be null"),
                        layout.size(),
                    );
                    new_node.use_subblock();
                    return Ok(slice_ptr);
                }
            }
        }
        return Err(AllocError);
    }

    fn deallocate(&mut self, ptr: NonNull<u8>) -> Option<AllocError> {
        let raw_mem_start = self.mem_start.as_ptr() as usize;
        let raw_ptr = ptr.as_ptr() as usize;
        let frame = (raw_ptr - raw_mem_start) / PAGE_FRAME_SIZE;

        let mut i = 0;
        let mut page_size_ref: Option<usize> = None;
        let mut valid_node_exists = false;
        // Find list head storing frame number
        for node in self.list_heads {
            if let Some(valid_node) = node {
                if valid_node.check_frame_used(frame) {
                    valid_node_exists = true;
                    page_size_ref = Some(SUBBLOCK_SIZES[i]);
                    break;
                }
            }
            i += 1;
        }

        // Find specific list node in linked list storing frame number
        let mut node_ref: Option<&mut ListNode> = None;
        if valid_node_exists {
            if let Some(list_head) = self.list_heads[i] {
                if list_head.this_frame == Some(frame) {
                    node_ref = Some(list_head);
                } else {
                    let mut stored_node = list_head;
                    while let Some(curr_node) = stored_node.next {
                        if curr_node.this_frame == Some(frame) {
                            node_ref = Some(curr_node);
                            break;
                        } else {
                            stored_node = curr_node;
                        }
                    }
                    if node_ref.is_none() {
                        panic!("Unknown error")
                    }
                }
            } else {
                panic!("Unknown error")
            }
        } else {
            // Assuming everything above is correct, a valid node would not
            // exist if frames were allocated whole
            self.fallback_allocator.dealloc(ptr);
        }


        if let Some(node) = node_ref {
            node.free_subblock();
            let bitmap_start_addr = node.end_addr();
            if let Some(page_size) = page_size_ref {
                let bitmap_size = (PAGE_FRAME_SIZE / page_size).div_ceil(8);
                let byte_index = frame / 8;
                let bit_index = frame % 8;
                unsafe {
                    let bitmap_slice = slice::from_raw_parts_mut(bitmap_start_addr, bitmap_size);
                    bitmap_slice[byte_index] = bitmap_slice[byte_index] & !(1 << bit_index);
                    let num_used_subblocks = (bitmap_size + size_of::<ListNode>()).div_ceil(page_size);
                    if Some(num_used_subblocks) == node.free_subblocks {
                        let slice_ptr = NonNull::new(node.start_addr() as *mut u8);
                        match slice_ptr {
                            Some(non_null) => {
                                self.fallback_allocator.dealloc(non_null);
                            }
                            None => {
                                panic!("Unknown error")
                            }
                        }
                    }
                }
            }
        }
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::prelude::*;

    #[test]
    fn test_list_node_new() {
        let node = ListNode::new();
        assert!(node.next.is_none());
        assert!(node.free_subblocks.is_none());
        assert!(node.this_frame.is_none());
        assert!(node.frames.is_none());
    }

    #[test]
    fn test_list_node_start_and_end_addr() {
        let node = ListNode::new();
        let start_addr = node.start_addr();
        let end_addr = node.end_addr();
        assert!(end_addr as usize > start_addr);
    }

    #[test]
    fn test_list_node_use_subblock() {
        let mut node = ListNode::new();
        assert_eq!(node.free_subblocks, None);
        node.use_subblock();
        assert_eq!(node.free_subblocks, Some(1));
        node.use_subblock();
        assert_eq!(node.free_subblocks, Some(2));
    }

    #[test]
    fn test_list_node_free_subblock() {
        let mut node = ListNode::new();
        node.free_subblocks = Some(2);
        node.free_subblock();
        assert_eq!(node.free_subblocks, Some(1));
        node.free_subblock();
        assert_eq!(node.free_subblocks, Some(0));
    }

    #[test]
    fn test_list_node_use_then_free_subblock() {
        let mut node = ListNode::new();
        assert_eq!(node.free_subblocks, None);
        node.use_subblock();
        assert_eq!(node.free_subblocks, Some(1));
        node.free_subblock();
        assert_eq!(node.free_subblocks, Some(0));
    }

    #[test]
    fn test_list_node_is_using_frame() {
        let mut bit_slice = bits![usize, Lsb0; 0; 8];
        bit_slice.set(3, true);
        let node = ListNode {
            frames: Some(bit_slice),
            ..ListNode::new()
        };

        assert!(node.is_using_frame(3));
        assert!(!node.is_using_frame(2));
    }

    #[test]
    fn test_list_node_set_frame_used_and_unused() {
        let mut node = ListNode::new();
        node.set_frame_used(2);
        assert!(node.check_frame_used(2));

        node.set_frame_unused(2);
        assert!(!node.check_frame_used(2));
    }
}