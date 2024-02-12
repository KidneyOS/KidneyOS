use core::{
    alloc::{Layout, Allocator},
    ptr::NonNull,
};
use crate::constants::KB;

#[derive(Clone, Copy)]
pub struct PoolAllocator<const N: usize> {
    region: NonNull<[u8]>,
}

impl<const N: usize> PoolAllocator<N>{
    /// Creates a new Pool Allocator that allocates a huge chunk of memory
    pub fn new(region: NonNull<[u8]>) -> Self {
        // Here, region is a Layout Pointer?
        // So we can use region to obtain the giant chunk of memory?
        todo!();
    }

}

unsafe impl<const N: usize> Allocator for PoolAllocator<N> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, core::alloc::AllocError> {
        // Tries to use the layout and self.region to find a potential next slot to allocate this region,
        // and return the pointer to it.
        todo!()
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