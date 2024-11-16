pub mod placement_algorithms;

use self::placement_algorithms::PlacementAlgorithm;

use super::FrameAllocator;
use alloc::boxed::Box;
use bitbybit::bitfield;
use core::alloc::AllocError;
use core::ptr::NonNull;
use kidneyos_shared::mem::PAGE_FRAME_SIZE;

#[bitfield(u8, default = 0)]
pub struct CoreMapEntry {
    #[bit(0, rw)]
    allocated: bool,
    #[bit(1, rw)]
    pinned: bool,
    #[bit(2, rw)]
    is_kernel: bool,
    #[bit(3, rw)]
    next: bool,
    #[bit(4, rw)]
    end: bool,
}

#[allow(clippy::type_complexity)]
pub struct FrameAllocatorSolution<A: PlacementAlgorithm> {
    start: NonNull<u8>,
    core_map: Box<[CoreMapEntry]>,
    frames_allocated: usize,
    placement_algorithm: A,
}

impl<A: PlacementAlgorithm> FrameAllocator for FrameAllocatorSolution<A> {
    fn alloc(&mut self, frames_requested: usize) -> Result<NonNull<[u8]>, AllocError> {
        if self.frames_allocated + frames_requested > self.core_map.len() {
            return Err(AllocError);
        }

        let range = self
            .placement_algorithm
            .place(&self.core_map, frames_requested)?;

        for i in range.clone() {
            assert!(!self.core_map[i].allocated());
            self.core_map[i] = self.core_map[i].with_next(true).with_allocated(true);

            if i == range.end - 1 {
                self.core_map[i] = self.core_map[i].with_end(true);
            }
        }

        self.frames_allocated += frames_requested;

        Ok(NonNull::slice_from_raw_parts(
            NonNull::new(unsafe { self.start.as_ptr().add(range.start * PAGE_FRAME_SIZE) })
                .ok_or(AllocError)?,
            range.len() * PAGE_FRAME_SIZE,
        ))
    }

    unsafe fn dealloc(&mut self, ptr_to_dealloc: NonNull<u8>) -> usize {
        let mut start =
            (ptr_to_dealloc.as_ptr() as usize - self.start.as_ptr() as usize) / PAGE_FRAME_SIZE;
        let mut frames_freed = 0;

        while start < self.core_map.len() {
            assert!(self.core_map[start].next());
            assert!(self.core_map[start].allocated());

            self.core_map[start] = self.core_map[start].with_next(false).with_allocated(false);

            self.frames_allocated -= 1;
            frames_freed += 1;

            if self.core_map[start].end() {
                self.core_map[start].with_end(false);
                break;
            }
            start += 1;
        }

        frames_freed
    }
}

impl<A: PlacementAlgorithm> FrameAllocatorSolution<A> {
    pub fn new(start: NonNull<u8>, core_map: Box<[CoreMapEntry]>) -> Self {
        FrameAllocatorSolution {
            start,
            core_map,
            frames_allocated: 0,
            placement_algorithm: Default::default(),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     use std::{
//         alloc::{Allocator, Global, Layout},
//         error::Error,
//     };
//
//     fn check_frames_allocated_position(
//         frame_allocator: &mut FrameAllocatorSolution,
//         expected_frames_allocated: usize,
//         expected_position: usize,
//     ) {
//         assert_eq!(frame_allocator.frames_allocated, expected_frames_allocated);
//         assert_eq!(frame_allocator.position, expected_position);
//     }
//     #[test]
//     fn test_alloc() -> Result<(), Box<dyn Error>> {
//         const NUM_FRAMES: usize = 16;
//
//         let core_map = [CoreMapEntry::DEFAULT; NUM_FRAMES];
//         let layout = Layout::from_size_align(PAGE_FRAME_SIZE * NUM_FRAMES, PAGE_FRAME_SIZE)?;
//         let region = Global.allocate(layout)?;
//
//         let mut frame_allocator =
//             FrameAllocatorSolution::new(region.cast::<u8>(), Box::new(core_map));
//
//         // Check that the frame allocator reports to have room for exactly 16 frames
//         assert!(frame_allocator.has_room(16));
//         assert!(!frame_allocator.has_room(17));
//
//         let allocation_1 = frame_allocator.alloc(1)?;
//
//         // Check that the first allocation returns the first frame
//         assert!(region.cast::<u8>() == allocation_1.cast::<u8>());
//         assert!(frame_allocator.core_map[0].allocated());
//         check_frames_allocated_position(&mut frame_allocator, 1, 1);
//
//         let allocation_2 = frame_allocator.alloc(5)?;
//
//         assert_eq!(allocation_2.cast::<u8>(), unsafe {
//             region.cast::<u8>().byte_add(PAGE_FRAME_SIZE)
//         });
//
//         // In total, we have allocated the 6 frames at the start of region
//         check_frames_allocated_position(&mut frame_allocator, 6, 6);
//         // Check that core map was also correctly updated
//         assert!(frame_allocator.has_room(10));
//         assert!(!frame_allocator.has_room(11));
//
//         let allocation_3 = frame_allocator.alloc(3)?;
//
//         assert_eq!(allocation_3.cast::<u8>(), unsafe {
//             region.cast::<u8>().byte_add(PAGE_FRAME_SIZE * 6)
//         });
//         // In total, we have allocated the 9 frames at the start of region
//         check_frames_allocated_position(&mut frame_allocator, 9, 9);
//
//         unsafe {
//             let deallocation_2 = frame_allocator.dealloc(allocation_2.cast::<u8>());
//             assert_eq!(deallocation_2, 5);
//         }
//         check_frames_allocated_position(&mut frame_allocator, 4, 9);
//
//         frame_allocator.set_placement_algorithm(placement_algorithms::best_fit);
//         let allocation_4 = frame_allocator.alloc(4)?;
//
//         assert_eq!(allocation_4.cast::<u8>(), unsafe {
//             region.cast::<u8>().byte_add(PAGE_FRAME_SIZE)
//         });
//         check_frames_allocated_position(&mut frame_allocator, 8, 5);
//
//         let allocation_5 = frame_allocator.alloc(1)?;
//
//         assert_eq!(allocation_5.cast::<u8>(), unsafe {
//             region.cast::<u8>().byte_add(PAGE_FRAME_SIZE * 5)
//         });
//         check_frames_allocated_position(&mut frame_allocator, 9, 6);
//
//         unsafe {
//             let deallocation_4 = frame_allocator.dealloc(allocation_4.cast::<u8>());
//             assert_eq!(deallocation_4, 4);
//         }
//         check_frames_allocated_position(&mut frame_allocator, 5, 6);
//
//         frame_allocator.set_placement_algorithm(placement_algorithms::first_fit);
//         let allocation_6 = frame_allocator.alloc(2)?;
//         assert_eq!(allocation_6.cast::<u8>(), unsafe {
//             region.cast::<u8>().byte_add(PAGE_FRAME_SIZE)
//         });
//         check_frames_allocated_position(&mut frame_allocator, 7, 3);
//
//         Ok(())
//     }
// }
