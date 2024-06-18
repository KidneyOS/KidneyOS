#![allow(unused_imports)]
use super::FrameAllocator;
use core::ptr::NonNull;
use core::{alloc::Allocator, ops::Range};

/// TODO
/// This is a temporary fix; need to figure out a way to pass in the number of max frames as an
/// initialization parameter
const TOTAL_FRAME_NUMBER: usize = 1000;

/*
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
            return None;
        }

        let res = self.first_unused_frame..self.first_unused_frame + frames;
        self.first_unused_frame += frames;
        Some(res)
    }

    fn dealloc(&mut self, _start: usize) {}
}
*/

/*
pub struct DummyAllocator{
    start_address: usize,
    end_address: usize,
}

impl DummyAllocator{
    fn new_in(start_address: usize, end_address: usize) -> DummyAllocator{
        DummyAllocator {
            start_address,
            end_address
        }
    }

    /*
    This is called when the kernel is first initialized to reserve room for the CoreMap structs
    and other page frame allocator. It will return a pointer to the
     */
    fn dummy_allocate(max_frames: usize) -> NonNull<[u8]>{

    }
}
*/

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub struct CoreMapEntry{
    allocated: bool,
    pinned: bool,
    is_kernel: bool,
    next: usize,
}

impl Default for CoreMapEntry{
    fn default() -> Self {
        CoreMapEntry{
            allocated: false,
            pinned: false,
            is_kernel: false,
            next: 0
        }
    }
}

pub struct FrameAllocatorSolution{
    core_map_entries: [CoreMapEntry; TOTAL_FRAME_NUMBER],
    placement_policy: PlacementPolicy,
    total_frames_allocated: usize,
    curr_position: usize,
}

#[allow(unused)]
pub enum PlacementPolicy{
    NextFit,
    FirstFit,
    BestFit,
}

impl FrameAllocator for FrameAllocatorSolution{
    fn new_in(placement_policy: PlacementPolicy) -> Self{
        let core_map_entries = [CoreMapEntry::default(); TOTAL_FRAME_NUMBER];
        FrameAllocatorSolution{
            core_map_entries,
            placement_policy,
            total_frames_allocated: 0,
            curr_position: 0
        }
    }

    fn alloc(&mut self, frames_requested: usize) -> Option<Range<usize>>{
        if self.total_frames_allocated + frames_requested > TOTAL_FRAME_NUMBER{
            return None;
        }

        match self.placement_policy{
            PlacementPolicy::NextFit => self.next_fit(frames_requested),
            PlacementPolicy::FirstFit => self.first_fit(frames_requested),
            PlacementPolicy::BestFit => self.best_fit(frames_requested),
        }
    }

    fn dealloc(&mut self, start: usize){
        let num_frames_to_free = self.core_map_entries[start].next + 1;

        for i in start..start + num_frames_to_free{
            assert_eq!(self.core_map_entries[i].allocated, true);
            assert_eq!(self.core_map_entries[i].next, num_frames_to_free + start - i - 1);

            self.core_map_entries[i].allocated = false;
            self.core_map_entries[i].next = 0;
        }

        self.total_frames_allocated -= num_frames_to_free;
    }
}

impl FrameAllocatorSolution{
    fn set_placement_policy(&mut self, new_placement_policy: PlacementPolicy){
        self.placement_policy = new_placement_policy;
    }

    fn next_fit(&mut self, frames_requested: usize) -> Option<Range<usize>>{
        for index in self.curr_position..self.curr_position + TOTAL_FRAME_NUMBER{
            let i = index % TOTAL_FRAME_NUMBER;

            if i + frames_requested > TOTAL_FRAME_NUMBER{
                continue;
            }

            let mut free_frames_found = 0;

            if !self.core_map_entries[i].allocated{
                free_frames_found += 1;

                for j in 1..frames_requested{
                    if !self.core_map_entries[i+j].allocated{
                        free_frames_found += 1;
                    }
                }
            }

            if free_frames_found == frames_requested{
                for k in i..i + frames_requested{
                    assert_eq!(self.core_map_entries[k].allocated, false);

                    self.core_map_entries[k].allocated = true;
                    self.core_map_entries[k].next = usize::from(frames_requested + i - k - 1);
                }

                self.curr_position = i + frames_requested;
                self.total_frames_allocated += frames_requested;

                return Some(i..i + frames_requested);
            }
        }

        None
    }

    fn first_fit(&mut self, frames_requested: usize) -> Option<Range<usize>> {
        for i in 0..=TOTAL_FRAME_NUMBER - frames_requested{
            let mut free_frames_found = 0;

            if !self.core_map_entries[i].allocated{
                free_frames_found += 1;

                for j in 1..frames_requested{
                    if !self.core_map_entries[i+j].allocated{
                        free_frames_found += 1;
                    }
                }
            }

            if free_frames_found == frames_requested{
                for k in i..i + frames_requested{
                    assert_eq!(self.core_map_entries[k].allocated, false);

                    self.core_map_entries[k].allocated = true;
                    self.core_map_entries[k].next = usize::from(frames_requested + i - k - 1);
                }

                self.total_frames_allocated += frames_requested;

                return Some(i..i + frames_requested);
            }
        }

        None
    }

    fn best_fit(&mut self, frames_requested: usize) -> Option<Range<usize>> {
        let mut best_start_index_so_far = TOTAL_FRAME_NUMBER;
        let mut best_chunk_size_so_far = TOTAL_FRAME_NUMBER + 1;
        let mut i = 0;

        while i < TOTAL_FRAME_NUMBER{
            if !self.core_map_entries[i].allocated{
                let start_index = i;
                let mut chunk_size = 0;

                while i < TOTAL_FRAME_NUMBER{
                    if self.core_map_entries[i].allocated{
                        break;
                    }

                    chunk_size += 1;
                    i += 1;
                }

                if chunk_size >= frames_requested{
                    if chunk_size - frames_requested < best_chunk_size_so_far{
                        best_chunk_size_so_far = chunk_size;
                        best_start_index_so_far = start_index;
                    }
                }

            } else {
                i += 1;
            }
        }

        if best_start_index_so_far == TOTAL_FRAME_NUMBER{
            return None;
        }

        for k in best_start_index_so_far..best_start_index_so_far + frames_requested{
            assert_eq!(self.core_map_entries[k].allocated, false);

            self.core_map_entries[k].allocated = true;
            self.core_map_entries[k].next = usize::from(frames_requested + best_start_index_so_far - k - 1);
        }

        self.total_frames_allocated += frames_requested;

        return Some(best_start_index_so_far..best_start_index_so_far + frames_requested);
    }
}

impl Default for FrameAllocatorSolution{
    fn default() -> Self {
        let core_map_entries = [CoreMapEntry::default(); TOTAL_FRAME_NUMBER];
        FrameAllocatorSolution{
            core_map_entries,
            placement_policy: PlacementPolicy::NextFit,
            total_frames_allocated: 0,
            curr_position: 0
        }
    }
}
