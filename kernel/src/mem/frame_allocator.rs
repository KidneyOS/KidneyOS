mod placement_algorithms;

use super::FrameAllocator;
use crate::mem::frame_allocator::placement_algorithms::next_fit;
use alloc::boxed::Box;
use bitbybit::bitfield;
use core::alloc::AllocError;
use core::{ops::Range, ptr::NonNull};
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
}

#[allow(clippy::type_complexity)]
pub struct FrameAllocatorSolution {
    start: NonNull<u8>,
    core_map: Box<[CoreMapEntry]>,
    placement_algorithm: fn(
        core_map: &[CoreMapEntry],
        frames_requested: usize,
        _position: usize,
    ) -> Result<Range<usize>, AllocError>,
    frames_allocated: usize,
    position: usize,
}

impl FrameAllocator for FrameAllocatorSolution {
    fn new_in(start: NonNull<u8>, core_map: Box<[CoreMapEntry]>) -> Self {
        FrameAllocatorSolution {
            start,
            core_map,
            placement_algorithm: next_fit,
            frames_allocated: 0,
            position: 0,
        }
    }

    fn alloc(&mut self, frames_requested: usize) -> Result<NonNull<[u8]>, AllocError> {
        if self.frames_allocated + frames_requested > self.core_map.len() {
            return Err(AllocError);
        }

        let range = (self.placement_algorithm)(&self.core_map, frames_requested, self.position)?;

        for i in range.clone() {
            assert!(!self.core_map[i].allocated());
            self.core_map[i] = self.core_map[i].with_next(true).with_allocated(true);
        }

        self.position = range.end;
        self.frames_allocated += frames_requested;

        Ok(NonNull::slice_from_raw_parts(
            NonNull::new(unsafe { self.start.as_ptr().add(range.start * PAGE_FRAME_SIZE) })
                .ok_or(AllocError)?,
            range.len() * PAGE_FRAME_SIZE,
        ))
    }

    fn dealloc(&mut self, ptr_to_dealloc: NonNull<u8>) -> usize {
        let start =
            (ptr_to_dealloc.as_ptr() as usize - self.start.as_ptr() as usize) / PAGE_FRAME_SIZE;
        let mut frames_freed = 0;

        while start < self.core_map.len() {
            if !self.core_map[start].next() {
                break;
            }
            assert!(self.core_map[start].next());
            assert!(self.core_map[start].allocated());

            self.core_map[start] = self.core_map[start].with_next(false).with_allocated(false);

            frames_freed += 1;
        }

        self.frames_allocated -= frames_freed;

        frames_freed
    }
}

impl FrameAllocatorSolution {
    pub fn has_room(&self, frames_requested: usize) -> bool {
        (self.placement_algorithm)(&self.core_map, frames_requested, self.position).is_ok()
    }
}
