pub mod placement_algorithms;

use self::placement_algorithms::PlacementAlgorithm;

use super::FrameAllocator;
use alloc::boxed::Box;
use core::alloc::AllocError;
use core::ptr::NonNull;
use kidneyos_shared::{bit_array::BitArray, bitfield, mem::PAGE_FRAME_SIZE};
use paste::paste;

bitfield!(
    CoreMapEntry, u8
    {}
    {
        (allocated, 0),
        (pinned, 1),
        (is_kernel, 2),
        (next, 3),
    }
);

#[allow(clippy::type_complexity)]
pub struct FrameAllocatorSolution<A: PlacementAlgorithm> {
    start: NonNull<[u8]>,
    core_map: Box<[CoreMapEntry]>,
    frames_allocated: usize,
    placement_algorithm: A,
}

impl<A> FrameAllocator for FrameAllocatorSolution<A>
where
    A: PlacementAlgorithm,
{
    fn alloc(&mut self, frames_requested: usize) -> Result<NonNull<u8>, AllocError> {
        if self.frames_allocated + frames_requested > self.core_map.len() {
            return Err(AllocError);
        }

        let range = self
            .placement_algorithm
            .place(&self.core_map, frames_requested)?;

        for i in range.clone() {
            assert!(!self.core_map[i].allocated());
            self.core_map[i] = self.core_map[i].with_allocated(true);

            if i != range.end - 1 {
                self.core_map[i] = self.core_map[i].with_next(true);
            }
        }

        self.frames_allocated += frames_requested;

        Ok(unsafe { self.start.cast::<u8>().add(range.start * PAGE_FRAME_SIZE) })
    }

    unsafe fn dealloc(&mut self, ptr_to_dealloc: NonNull<u8>) -> usize {
        let mut start = (ptr_to_dealloc.as_ptr() as usize
            - self.start.cast::<u8>().as_ptr() as usize)
            / PAGE_FRAME_SIZE;
        let mut frames_freed = 1;

        while self.core_map[start].next() {
            assert!(self.core_map[start].allocated());
            self.core_map[start] = self.core_map[start].with_next(false).with_allocated(false);

            frames_freed += 1;
            start += 1;
        }

        self.core_map[start] = self.core_map[start].with_allocated(false);

        self.frames_allocated -= frames_freed;
        frames_freed
    }
}

impl<A: PlacementAlgorithm> FrameAllocatorSolution<A> {
    pub fn new(start: NonNull<[u8]>, core_map: Box<[CoreMapEntry]>) -> Self {
        FrameAllocatorSolution {
            start,
            core_map,
            frames_allocated: 0,
            placement_algorithm: Default::default(),
        }
    }
}

impl<A> FrameAllocatorSolution<A>
where
    A: PlacementAlgorithm,
{
    #[allow(dead_code)]
    pub fn num_allocated(&self) -> usize {
        self.frames_allocated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::mem::frame_allocator::placement_algorithms::{BestFit, FirstFit, NextFit};
    use core::ops::Range;
    use std::{
        alloc::{Allocator, Global, Layout},
        error::Error,
    };

    fn check_coremap(core_map: &[CoreMapEntry], indices: Range<usize>, check: bool) {
        for i in indices.clone() {
            assert_eq!(core_map[i].allocated(), check);

            if i != indices.end - 1 {
                assert_eq!(core_map[i].next(), check);
            }
        }
        assert!(!core_map[indices.end - 1].next());
    }

    // All placement policies should allocate the same frames for the below allocations
    fn setup<A>(frame_allocator: &mut FrameAllocatorSolution<A>, region: &NonNull<[u8]>)
    where
        A: PlacementAlgorithm,
    {
        let allocation_1 = frame_allocator.alloc(3).unwrap();
        assert_eq!(allocation_1.cast::<u8>(), region.cast::<u8>());
        assert_eq!(frame_allocator.frames_allocated, 3);
        check_coremap(&frame_allocator.core_map, 0..3, true);

        let allocation_2 = frame_allocator.alloc(4).unwrap();
        assert_eq!(allocation_2.cast::<u8>(), unsafe {
            region.cast::<u8>().byte_add(PAGE_FRAME_SIZE * 3)
        });
        assert_eq!(frame_allocator.frames_allocated, 7);
        check_coremap(&frame_allocator.core_map, 3..7, true);

        let allocation_3 = frame_allocator.alloc(5).unwrap();
        assert_eq!(allocation_3.cast::<u8>(), unsafe {
            region.cast::<u8>().byte_add(PAGE_FRAME_SIZE * 7)
        });
        assert_eq!(frame_allocator.frames_allocated, 12);
        check_coremap(&frame_allocator.core_map, 7..12, true);

        let allocation_4 = frame_allocator.alloc(1).unwrap();
        assert_eq!(allocation_4.cast::<u8>(), unsafe {
            region.cast::<u8>().byte_add(PAGE_FRAME_SIZE * 12)
        });
        assert_eq!(frame_allocator.frames_allocated, 13);
        check_coremap(&frame_allocator.core_map, 12..13, true);

        let allocation_5 = frame_allocator.alloc(2).unwrap();
        assert_eq!(allocation_5.cast::<u8>(), unsafe {
            region.cast::<u8>().byte_add(PAGE_FRAME_SIZE * 13)
        });
        assert_eq!(frame_allocator.frames_allocated, 15);
        check_coremap(&frame_allocator.core_map, 13..15, true);

        unsafe {
            let deallocation_2 = frame_allocator.dealloc(allocation_2.cast::<u8>());
            assert_eq!(deallocation_2, 4);
        };

        assert_eq!(frame_allocator.frames_allocated, 11);
        check_coremap(&frame_allocator.core_map, 3..7, false);

        unsafe {
            let deallocation_4 = frame_allocator.dealloc(allocation_4.cast::<u8>());
            assert_eq!(deallocation_4, 1);
        };

        assert_eq!(frame_allocator.frames_allocated, 10);
        check_coremap(&frame_allocator.core_map, 12..13, false);
    }

    #[test]
    fn test_alloc_next_fit() -> Result<(), Box<dyn Error>> {
        const NUM_FRAMES: usize = 18;

        let core_map = [CoreMapEntry::default(); NUM_FRAMES];
        let layout = Layout::from_size_align(PAGE_FRAME_SIZE * NUM_FRAMES, PAGE_FRAME_SIZE)?;
        let region = Global.allocate(layout)?;

        let mut frame_allocator =
            FrameAllocatorSolution::<NextFit>::new(region, Box::new(core_map));

        assert_eq!(
            unsafe { frame_allocator.start.as_ref().len() },
            PAGE_FRAME_SIZE * NUM_FRAMES
        );

        setup(&mut frame_allocator, &region);

        let diff_alloc = frame_allocator.alloc(1)?;
        assert_eq!(diff_alloc.cast::<u8>(), unsafe {
            region.cast::<u8>().byte_add(PAGE_FRAME_SIZE * 15)
        });
        assert_eq!(frame_allocator.frames_allocated, 11);
        check_coremap(&frame_allocator.core_map, 15..16, true);

        Ok(())
    }

    #[test]
    fn test_alloc_first_fit() -> Result<(), Box<dyn Error>> {
        const NUM_FRAMES: usize = 18;

        let core_map = [CoreMapEntry::default(); NUM_FRAMES];
        let layout = Layout::from_size_align(PAGE_FRAME_SIZE * NUM_FRAMES, PAGE_FRAME_SIZE)?;
        let region = Global.allocate(layout)?;

        let mut frame_allocator =
            FrameAllocatorSolution::<FirstFit>::new(region, Box::new(core_map));

        assert_eq!(
            unsafe { frame_allocator.start.as_ref().len() },
            PAGE_FRAME_SIZE * NUM_FRAMES
        );

        setup(&mut frame_allocator, &region);

        let diff_alloc = frame_allocator.alloc(1)?;
        assert_eq!(diff_alloc.cast::<u8>(), unsafe {
            region.cast::<u8>().byte_add(PAGE_FRAME_SIZE * 3)
        });
        assert_eq!(frame_allocator.frames_allocated, 11);
        check_coremap(&frame_allocator.core_map, 3..4, true);

        Ok(())
    }

    #[test]
    fn test_alloc_best_fit() -> Result<(), Box<dyn Error>> {
        const NUM_FRAMES: usize = 18;

        let core_map = [CoreMapEntry::default(); NUM_FRAMES];
        let layout = Layout::from_size_align(PAGE_FRAME_SIZE * NUM_FRAMES, PAGE_FRAME_SIZE)?;
        let region = Global.allocate(layout)?;

        let mut frame_allocator =
            FrameAllocatorSolution::<BestFit>::new(region, Box::new(core_map));

        assert_eq!(
            unsafe { frame_allocator.start.as_ref().len() },
            PAGE_FRAME_SIZE * NUM_FRAMES
        );

        setup(&mut frame_allocator, &region);

        let diff_alloc = frame_allocator.alloc(1)?;
        assert_eq!(diff_alloc.cast::<u8>(), unsafe {
            region.cast::<u8>().byte_add(PAGE_FRAME_SIZE * 12)
        });
        assert_eq!(frame_allocator.frames_allocated, 11);
        check_coremap(&frame_allocator.core_map, 12..13, true);

        Ok(())
    }
}
