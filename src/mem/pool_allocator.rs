use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::size_of,
    ptr,
    ptr::NonNull,
    slice,
};

pub struct PoolAllocator<const N: usize> {
    region: NonNull<[u8]>,
    // bitmap: UnsafeCell<Box<[u8]>>,
}

impl<const N: usize> PoolAllocator<N> {
    /// PoolAllocator has the giant chunk of memory referred to in region
    pub fn new(region: NonNull<[u8]>) -> Self {
        // Ensure the region is large enough to store at least N bytes,
        // because we're handing out N-byte blocks
        assert!(region.len() >= N);
        assert!(N.is_power_of_two()); // N must be a power of 2

        // Calculate the required bitmap size in bytes, because each stores an u8

        // Round up the division to the nearest whole number
        // This unit is in bytes
        let bitmap_size = (region.len() / N).div_ceil(8);

        // To allow the first block to always store the size of bitmap
        assert!(N >= size_of::<usize>());

        // Calculate how many blocks the bitmap occupies
        let bitmap_blocks = bitmap_size.div_ceil(N);
        let total_blocks = region.len().div_ceil(N);

        // Ensure the region is large enough for the size of bitmap + bitmap_blocks
        // + at least 1 extra block is available
        assert!(total_blocks >= bitmap_blocks + 1 + 1);

        unsafe {
            let region_ptr = region.as_ptr();

            // The first block is used to store how large the bitmap is, in units of bytes
            ptr::write(region_ptr.cast::<usize>(), bitmap_size);

            // Initialize the bitmap area to zero, which is 1 block away from the start
            ptr::write_bytes(region_ptr.cast::<u8>().add(N), 0, bitmap_size);

            // Get a mutable slice of the region where the bitmap is stored
            let bitmap_slice =
                slice::from_raw_parts_mut(region_ptr.cast::<u8>().add(N), bitmap_size);

            // Mark the blocks used by the bitmap as used in the bitmap
            for i in 0..bitmap_blocks {
                let byte_index = i / 8;
                let bit_index = i % 8;
                if byte_index < bitmap_size {
                    // Set the bit to mark the block as used
                    bitmap_slice[byte_index] |= 1 << bit_index;
                }
            }
        }

        Self { region }
    }
}

unsafe impl<const N: usize> Allocator for PoolAllocator<N> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Can only allocate exactly of size N
        if layout.size() != N {
            return Err(AllocError);
        }

        // Check alignment of layout
        if layout.align() <= N {
            return Err(AllocError);
        }

        // Calculate the number of blocks required
        let blocks_required = layout.size() / N;

        // Variables to track the search for a contiguous free region
        let mut start_index = None;
        let mut free_count = 0;

        // Obtain the bitmap from the region
        // Size of the bitmap is in the first block
        let bitmap_size = unsafe { *(self.region.as_ptr() as *const usize) };

        // Get a pointer to the start of the bitmap, the bitmap starts 1 block away from the start
        let bitmap_ptr = unsafe { (self.region.as_ptr() as *const u8).add(N) };

        // Search for a contiguous sequence of free blocks
        for i in 0..bitmap_size {
            unsafe {
                for j in 0..8 {
                    let bit = bitmap_ptr.add(i).read() & (1 << j);
                    if bit == 0 {
                        free_count += 1;
                        if free_count == blocks_required {
                            start_index = Some(i * 8 + j - free_count + 1);
                            break;
                        }
                    } else {
                        free_count = 0;
                    }
                }
            }
            if start_index.is_some() {
                break;
            }
        }

        // Didn't find any suitable region
        let start_bit = start_index.ok_or(AllocError)?;

        // Not enough free blocks possible
        if free_count < blocks_required {
            return Err(AllocError);
        }

        // Calculate the start address
        let start_addr = unsafe { (self.region.as_ptr() as *const u8).add(start_bit * N) };

        // Update the bitmap to mark the blocks as used
        unsafe {
            // Get the bitmap pointer
            let bitmap_ptr = self.region.as_ptr().cast::<u8>().add(N);

            for i in 0..blocks_required {
                let byte_index = (start_bit + i) / 8;
                let bit_pos = (start_bit + i) % 8;
                bitmap_ptr
                    .add(byte_index)
                    .write(bitmap_ptr.add(byte_index).read() | (1 << bit_pos));
            }
        }

        // Construct and return the pointer to the allocated memory
        let slice_ptr = NonNull::slice_from_raw_parts(
            NonNull::new(start_addr as *mut u8).unwrap(),
            layout.size(),
        );

        let nonnull_slice = NonNull::new(slice_ptr.as_ptr()).unwrap();

        Ok(nonnull_slice)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        // Using the bitmap to check if the ptr is aligned with our allocation state
        let region_slice = unsafe { &*self.region.as_ptr() };

        let start_addr = region_slice.as_ptr() as usize; // Start address of the region

        // Get a pointer to the start of the bitmap, the bitmap starts 1 block away from the start
        let bitmap_ptr = self.region.as_ptr().cast::<u8>().add(N);

        // Find out which block it belongs to
        let start_bit = (ptr.as_ptr() as usize - start_addr) / N;

        // Sanity check: We should have layout.size() / N blocks starting from start_bit
        for i in 0..layout.size().div_ceil(N) {
            let byte_index = (start_bit + i) / 8;
            let bit_pos = (start_bit + i) % 8;
            if bitmap_ptr.add(byte_index).read() & (1 << bit_pos) == 0 {
                panic!("Double free detected");
            }
        }

        // Mark the blocks as free
        for i in 0..layout.size().div_ceil(N) {
            let byte_index = (start_bit + i) / 8;
            let bit_pos = (start_bit + i) % 8;
            bitmap_ptr
                .add(byte_index)
                .write(bitmap_ptr.add(byte_index).read() & !(1 << bit_pos));
        }
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
