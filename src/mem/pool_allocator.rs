use core::{
    alloc::{Layout, Allocator, AllocError},
    ptr::NonNull,
    cell::UnsafeCell,
};

use alloc::vec::Vec;
use alloc::vec;

pub struct PoolAllocator<const N: usize> {
    region: NonNull<[u8]>,
    bitmap: UnsafeCell<Vec<u8>>,
}

impl<const N: usize> PoolAllocator<N>{
    /// PoolAllocator has the giant chunk of memory referred to in region
    pub fn new(region: NonNull<[u8]>) -> Self {
        // Calculate the required bitmap size in bytes, because each stores an u8

        // We will round down to ensure that it doesn't crash
        // It will always use lesser than or equal to the region size
        let bitmap_size = (region.len() / N) / 8;

        // Initialize the bitmap vector with zeros
        let bitmap = UnsafeCell::new(vec![0u8; bitmap_size]);

        Self { region, bitmap }
    }

}

unsafe impl<const N: usize> Allocator for PoolAllocator<N> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, core::alloc::AllocError> {
        // Ensure the layout size is a multiple of N
        if layout.size() % N != 0 {
            return Err(AllocError);
        }

        // Calculate the number of blocks required
        let blocks_required = layout.size() / N;

        // Variables to track the search for a contiguous free region
        let mut start_index = None;
        let mut free_count = 0;

        let bitmap = unsafe { self.bitmap.get().as_mut() }.unwrap();

        // Search for a contiguous sequence of free blocks
        for (index, &bit) in bitmap.iter().enumerate(){
            for bit_pos in 0..8 {
                if bit & (1 << bit_pos) == 0 {
                    free_count += 1;
                    start_index.get_or_insert(index * 8 + bit_pos);
                    if free_count >= blocks_required {
                        // Found a suitable region
                        break;
                    }
                } else {
                    // Reset the counter and start index if a used block is found
                    start_index = None;
                    free_count = 0;
                }
            }
        }

        // Didn't find any suitable region
        if start_index.is_none() {
            return Err(AllocError);
        }

        let start_bit: usize = start_index.unwrap();

        if free_count >= blocks_required {
            // Calculate the start address
            let start_addr = unsafe {
                (self.region.as_ptr() as *const u8)
                    .add(start_bit * N)
            };

            // Update the bitmap to mark the blocks as used
            unsafe {
                let bitmap_ptr = self.bitmap.get().as_mut().unwrap(); // Get a mutable reference to the bitmap
                for i in 0..blocks_required {
                    let byte_index = (start_bit + i) / 8;
                    let bit_pos = (start_bit + i) % 8;
                    (*bitmap_ptr)[byte_index] |= 1 << bit_pos;
                }
            }

            // Construct and return the pointer to the allocated memory
            let slice_ptr = NonNull::slice_from_raw_parts(
                NonNull::new(start_addr as *mut u8).unwrap(),
                layout.size());

            let nonnull_slice = NonNull::new(slice_ptr.as_ptr() as *mut [u8]).unwrap();

            Ok(nonnull_slice)
        } else {
            Err(AllocError)
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        // Marks some pointer region as unused, might need more thinking before writing it up,
        // otherwise might need to also allow each pointer location to store a "used/unused" information
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use std::error::Error;

    // Importing pool allocator for testing
    use crate::mem::pool_allocator::PoolAllocator;

    #[test]
    fn pool_allocator_simple() -> Result<(), Box<dyn Error>> {
        todo!();
    }
}