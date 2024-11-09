use super::FrameAllocator;
use alloc::boxed::Box;
use bitbybit::bitfield;
use core::{
    alloc::AllocError,
    sync::atomic::{AtomicUsize, Ordering},
};
use core::{ops::Range, ptr::NonNull};
use kidneyos_shared::mem::PAGE_FRAME_SIZE;

// For replacement policy and bookkeeping
static CURR_NUM_FRAMES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static CURR_POSITION: AtomicUsize = AtomicUsize::new(0);

// TODO: Verify the correctness of all placement policy algorithms
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

pub struct FrameAllocatorSolution {
    start: NonNull<u8>,
    core_map: Box<[CoreMapEntry]>,
    placement_policy: PlacementPolicy,
    total_number_of_frames: usize,
}

#[allow(unused)]
#[allow(clippy::enum_variant_names)]
pub enum PlacementPolicy {
    NextFit,
    FirstFit,
    BestFit,
}

impl FrameAllocator for FrameAllocatorSolution {
    fn new_in(
        start: NonNull<u8>,
        core_map: Box<[CoreMapEntry]>,
        total_number_of_frames: usize,
    ) -> Self {
        FrameAllocatorSolution {
            start,
            core_map,
            placement_policy: PlacementPolicy::NextFit,
            total_number_of_frames,
        }
    }
    
    fn alloc(&mut self, frames_requested: usize) -> Result<NonNull<[u8]>, AllocError> {
        if CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + frames_requested
            > self.total_number_of_frames
        {
            return Err(AllocError);
        }

        let Some(range) = (match self.placement_policy {
            PlacementPolicy::NextFit => self.next_fit(frames_requested),
            PlacementPolicy::FirstFit => self.first_fit(frames_requested),
            PlacementPolicy::BestFit => self.best_fit(frames_requested),
        }) else {
            return Err(AllocError);
        };

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

        while start < self.total_number_of_frames {
            if !self.core_map[start].next() {
                break;
            }
            assert!(self.core_map[start].next());
            assert!(self.core_map[start].allocated());

            self.core_map[start] = self.core_map[start].with_next(false).with_allocated(false);

            num_frames_to_free += 1;
        }

        let temp = CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) - num_frames_to_free;
        CURR_NUM_FRAMES_ALLOCATED.store(temp, Ordering::Relaxed);

        num_frames_to_free
    }
}

impl FrameAllocatorSolution {
    #[allow(dead_code)]
    pub fn set_placement_policy(&mut self, new_placement_policy: PlacementPolicy) {
        self.placement_policy = new_placement_policy;
    }

    pub fn has_room(&self, frames_requested: usize) -> bool {
        if CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + frames_requested
            > self.total_number_of_frames
        {
            return false;
        };

        let mut i = 0;
        let mut largest_chunk = 0;

        while i < self.total_number_of_frames {
            if !self.core_map[i].allocated() {
                let mut chunk_size = 0;

                while i < self.total_number_of_frames {
                    if self.core_map[i].next() {
                        break;
                    }

                    chunk_size += 1;
                    i += 1;
                }

                if chunk_size > largest_chunk {
                    largest_chunk = chunk_size
                }
            } else {
                i += 1;
            }
        }

        largest_chunk >= frames_requested
    }

    fn next_fit(&mut self, frames_requested: usize) -> Option<Range<usize>> {
        for index in CURR_POSITION.load(Ordering::Relaxed)
            ..CURR_POSITION.load(Ordering::Relaxed) + self.total_number_of_frames
        {
            let i = index % self.total_number_of_frames;

            if i + frames_requested > self.total_number_of_frames {
                continue;
            }

            let mut free_frames_found = 0;

            if !self.core_map[i].allocated() {
                free_frames_found += 1;

                for j in 1..frames_requested {
                    if !self.core_map[i + j].allocated() {
                        free_frames_found += 1;
                    }
                }
            }

            if free_frames_found == frames_requested {
                for k in i..i + frames_requested {
                    assert!(!self.core_map[k].allocated());

                    self.core_map[k] = self.core_map[k].with_next(true).with_allocated(true);
                }

                CURR_POSITION.store(i + frames_requested, Ordering::Relaxed);
                let temp = CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + frames_requested;
                CURR_NUM_FRAMES_ALLOCATED.store(temp, Ordering::Relaxed);

                return Some(i..i + frames_requested);
            }
        }

        None
    }

    fn first_fit(&mut self, frames_requested: usize) -> Option<Range<usize>> {
        for i in 0..=self.total_number_of_frames - frames_requested {
            let mut free_frames_found = 0;

            if !self.core_map[i].allocated() {
                free_frames_found += 1;

                for j in 1..frames_requested {
                    if !self.core_map[i + j].allocated() {
                        free_frames_found += 1;
                    }
                }
            }

            if free_frames_found == frames_requested {
                for k in i..i + frames_requested {
                    assert!(!self.core_map[k].allocated());

                    self.core_map[k] = self.core_map[k].with_next(true).with_allocated(true);
                }

                let temp = CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + frames_requested;
                CURR_NUM_FRAMES_ALLOCATED.store(temp, Ordering::Relaxed);

                return Some(i..i + frames_requested);
            }
        }

        None
    }

    fn best_fit(&mut self, frames_requested: usize) -> Option<Range<usize>> {
        let mut best_start_index_so_far = self.total_number_of_frames;
        let mut best_chunk_size_so_far = self.total_number_of_frames + 1;
        let mut i = 0;

        while i < self.total_number_of_frames {
            if !self.core_map[i].allocated() {
                let start_index = i;
                let mut chunk_size = 0;

                while i < self.total_number_of_frames {
                    if self.core_map[i].allocated() {
                        break;
                    }

                    chunk_size += 1;
                    i += 1;
                }

                if chunk_size >= frames_requested
                    && chunk_size - frames_requested < best_chunk_size_so_far
                {
                    best_chunk_size_so_far = chunk_size;
                    best_start_index_so_far = start_index;
                }
            } else {
                i += 1;
            }
        }

        if best_start_index_so_far == self.total_number_of_frames {
            return None;
        }

        for k in best_start_index_so_far..best_start_index_so_far + frames_requested {
            assert!(!self.core_map[k].allocated());
            self.core_map[k] = self.core_map[k].with_next(true).with_allocated(true);
        }

        let temp = CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + frames_requested;
        CURR_NUM_FRAMES_ALLOCATED.store(temp, Ordering::Relaxed);

        Some(best_start_index_so_far..best_start_index_so_far + frames_requested)
    }
}
