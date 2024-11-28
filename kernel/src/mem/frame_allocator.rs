use super::FrameAllocator;
use crate::swapping::page_replacement::{PageReplacementPolicy, RandomEviction};
use core::{alloc::Allocator, ops::Range};

pub struct FrameAllocatorSolution<A>
where
    A: Allocator,
{
    #[allow(unused)]
    alloc: A,
    max_frames: usize,
    first_unused_frame: usize,
}

// SAFETY: This frame allocator never hands out references to the same page
// twice.
unsafe impl<A> FrameAllocator<A> for FrameAllocatorSolution<A>
where
    A: Allocator,
{
    fn new_in(alloc: A, max_frames: usize) -> Self
    where
        Self: Sized,
    {
        Self {
            alloc,
            max_frames,
            first_unused_frame: 0,
        }
    }

    fn alloc(&mut self, frames: usize) -> Option<Range<usize>> {
        if self.first_unused_frame + frames >= self.max_frames {
            // Evict a page and return
            let _evicted_frame = RandomEviction::evict_page(self.max_frames);
        }

        let res = self.first_unused_frame..self.first_unused_frame + frames;
        self.first_unused_frame += frames;
        Some(res)
    }

    fn dealloc(&mut self, _start: usize) {}
}
