mod placement_algorithms;

use super::FrameAllocator;
use crate::mem::frame_allocator::placement_algorithms::next_fit;
use alloc::boxed::Box;
use bitbybit::bitfield;
use core::{
    alloc::AllocError,
    sync::atomic::{AtomicUsize, Ordering},
};
use core::{ops::Range, ptr::NonNull};
use kidneyos_shared::mem::PAGE_FRAME_SIZE;

static CURR_NUM_FRAMES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static CURR_POSITION: AtomicUsize = AtomicUsize::new(0);

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
    placement_algorithm: fn(&[CoreMapEntry], usize, usize) -> Result<Range<usize>, AllocError>,
}

impl FrameAllocator for FrameAllocatorSolution {
    fn new_in(start: NonNull<u8>, core_map: Box<[CoreMapEntry]>) -> Self {
        FrameAllocatorSolution {
            start,
            core_map,
            placement_algorithm: next_fit,
        }
    }

    fn alloc(&mut self, frames_requested: usize) -> Result<NonNull<[u8]>, AllocError> {
        if CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + frames_requested
            > self.core_map.len()
        {
            return Err(AllocError);
        }

        let range = (self.placement_algorithm)(
            &self.core_map,
            frames_requested,
            CURR_POSITION.load(Ordering::Relaxed),
        )?;

        for i in range.clone() {
            assert!(!self.core_map[i].allocated());
            self.core_map[i] = self.core_map[i].with_next(true).with_allocated(true);
        }

        CURR_POSITION.store(range.end, Ordering::Relaxed);
        CURR_NUM_FRAMES_ALLOCATED.fetch_add(frames_requested, Ordering::Relaxed);

        Ok(NonNull::slice_from_raw_parts(
            NonNull::new(unsafe { self.start.as_ptr().add(range.start * PAGE_FRAME_SIZE) })
                .ok_or(AllocError)?,
            range.len() * PAGE_FRAME_SIZE,
        ))
    }

    fn dealloc(&mut self, ptr_to_dealloc: NonNull<u8>) -> usize {
        let start =
            (ptr_to_dealloc.as_ptr() as usize - self.start.as_ptr() as usize) / PAGE_FRAME_SIZE;
        let mut num_frames_to_free = 0;

        while start < self.core_map.len() {
            if !self.core_map[start].next() {
                break;
            }
            assert!(self.core_map[start].next());
            assert!(self.core_map[start].allocated());

            self.core_map[start] = self.core_map[start].with_next(false).with_allocated(false);

            num_frames_to_free += 1;
        }

        CURR_NUM_FRAMES_ALLOCATED.fetch_sub(num_frames_to_free, Ordering::Relaxed);

        num_frames_to_free
    }
}

impl FrameAllocatorSolution {
    pub fn has_room(&self, frames_requested: usize) -> bool {
        (self.placement_algorithm)(
            &self.core_map,
            frames_requested,
            CURR_POSITION.load(Ordering::Relaxed),
        )
        .is_ok()
    }
}
