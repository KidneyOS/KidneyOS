use super::FrameAllocator;
use core::{alloc::Allocator, ops::Range};
use kidneyos_shared::mem::mem_addr_types::VirtAddr;

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

    fn alloc(&mut self, frames: usize) -> Option<Range<VirtAddr>> {
        if self.first_unused_frame + frames >= self.max_frames {
            return None;
        }

        let res = self.first_unused_frame..self.first_unused_frame + frames;
        self.first_unused_frame += frames;
        Some(res)
    }

    fn dealloc(&mut self, _start: VirtAddr) {}
}
