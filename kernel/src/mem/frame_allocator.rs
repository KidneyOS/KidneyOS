use super::FrameAllocator;
use core::{
    ptr::NonNull,
    alloc::Allocator,
    ops::Range
};
use std::{
    alloc::AllocError,
    hash::RandomState,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering}
};

use bitbybit::bitfield;
use kidneyos_shared::mem::PAGE_FRAME_SIZE;

// For replacement policy and bookkeeping
static CURR_NUM_FRAMES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static CURR_POSITION: AtomicUsize = AtomicUsize::new(0);

pub struct DummyAllocatorSolution{
    start_address: usize,
    end_address: usize,
}

impl DummyAllocatorSolution{
    pub fn new_in(start_address: usize, end_address: usize) -> Self{
        DummyAllocatorSolution{
            start_address,
            end_address
        }
    }

    /*
    Dummy Allocator does 2 things
    1. Returns a piece of memory used to store the CoreMap Entries for actual frames
    2. Increments the start address by the number of frames the CoreMap Entries (we do this
    because we never want to free the region of memory storing the CoreMap Entries)
    */
    pub fn alloc(&mut self, frames_requested: usize) -> Result<NonNull<[u8]>, AllocError>{
        // Don't think this will ever happen, but good to have a check for it
        if self.start_address + (PAGE_FRAME_SIZE * frames_requested) > self.end_address{
            return Err(AllocError);
        }

        let new_addr = (self.start_address + (PAGE_FRAME_SIZE * frames_requested))
            .next_multiple_of(PAGE_FRAME_SIZE);

        let ret = Ok(NonNull::slice_from_raw_parts(
            NonNull::new(unsafe { self.start_address as *mut u8 })
                .ok_or(AllocError)?,
            frames_requested * PAGE_FRAME_SIZE,
        ));

        self.start_address = new_addr;
        ret
    }

    pub fn get_start_address(&self) -> usize{
        self.start_address
    }

    pub fn get_end_address(&self) -> usize{
        self.end_address
    }

    pub fn set_start_address(&mut self, new_start: usize){
        self.start_address = new_start;
    }

    pub fn set_end_address(&mut self, new_end: usize){
        self.end_address = new_end;
    }
}

// TODO: Verify the correctness of all placement policy algorithms
#[bitfield(u8, default=0)]
pub struct CoreMapEntry{
    #[bit(0, rw)]
    allocated: bool,
    #[bit(1, rw)]
    pinned: bool,
    #[bit(2, rw)]
    is_kernel: bool,
    #[bit(3, rw)]
    next: bool,
}

pub struct FrameAllocatorSolution{
    start: NonNull<u8>,
    core_map: Box<[CoreMapEntry]>,
    placement_policy: PlacementPolicy,
    total_number_of_frames: usize,
}

#[allow(unused)]
pub enum PlacementPolicy{
    NextFit,
    FirstFit,
    BestFit,
}

unsafe impl FrameAllocator for FrameAllocatorSolution{
    fn new_in(start: NonNull<u8>, core_map: Box<[CoreMapEntry]>,
              total_number_of_frames: usize) -> Self{
        FrameAllocatorSolution{
            start,
            core_map,
            placement_policy: PlacementPolicy::NextFit,
            total_number_of_frames,
        }
    }

    fn alloc(&mut self, frames_requested: usize) -> Result<NonNull<[u8]>, AllocError>{
        if CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + frames_requested > self.total_number_of_frames {
            return Err(AllocError);
        }

        let Some(range) = match self.placement_policy{
            PlacementPolicy::NextFit => self.next_fit(frames_requested),
            PlacementPolicy::FirstFit => self.first_fit(frames_requested),
            PlacementPolicy::BestFit => self.best_fit(frames_requested),
        } else {
            return Err(AllocError);
        };

        Ok(NonNull::slice_from_raw_parts(
            NonNull::new(unsafe { self.start.as_ptr().add(range.start * PAGE_FRAME_SIZE) })
                .ok_or(AllocError)?,
            range.len() * PAGE_FRAME_SIZE,
        ))
    }

    fn dealloc(&mut self, ptr_to_dealloc: NonNull<u8>) -> usize{
        let start = (ptr_to_dealloc.as_ptr() as usize - self.start.as_ptr() as usize) / PAGE_FRAME_SIZE;
        let mut num_frames_to_free = 0;

        while start < self.total_number_of_frames {
            if !self.core_map[start].bit3(){
                break;
            }
            assert_eq!(self.core_map[start].bit3(), true);
            assert_eq!(self.core_map[start].bit0(), true);

            self.core_map[start].bit3() = false;
            self.core_map[start].bit0() = false;

            num_frames_to_free += 1;
        }

        let temp = CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) - num_frames_to_free;
        CURR_NUM_FRAMES_ALLOCATED.store(temp, Ordering::Relaxed);

        num_frames_to_free
    }
}

// TODO: All of these should just return a physical address to the start of the frame
impl FrameAllocatorSolution{
    fn set_placement_policy(&mut self, new_placement_policy: PlacementPolicy){
        self.placement_policy = new_placement_policy;
    }

    fn next_fit(&mut self, frames_requested: usize) -> Option<Range<usize>>{
        for index in CURR_POSITION.load(Ordering::Relaxed)..
            CURR_POSITION.load(Ordering::Relaxed) + self.total_number_of_frames {
            let i = index % self.total_number_of_frames;

            if i + frames_requested > self.total_number_of_frames {
                continue;
            }

            let mut free_frames_found = 0;

            if !self.core_map[i].bit0(){
                free_frames_found += 1;

                for j in 1..frames_requested{
                    if !self.core_map[i+j].bit0(){
                        free_frames_found += 1;
                    }
                }
            }

            if free_frames_found == frames_requested{
                for k in i..i + frames_requested{
                    assert_eq!(self.core_map[k].bit0(), false);

                    self.core_map[k].bit0() = true;
                    self.core_map[k].bit3() = true;
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
        for i in 0..=self.total_number_of_frames - frames_requested{
            let mut free_frames_found = 0;

            if !self.core_map[i].bit0(){
                free_frames_found += 1;

                for j in 1..frames_requested{
                    if !self.core_map[i+j].bit0(){
                        free_frames_found += 1;
                    }
                }
            }

            if free_frames_found == frames_requested{
                for k in i..i + frames_requested{
                    assert_eq!(self.core_map[k].bit0(), false);

                    self.core_map[k].bit0() = true;
                    self.core_map[k].bit3() = true;
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
            if !self.core_map[i].bit0(){
                let start_index = i;
                let mut chunk_size = 0;

                while i < self.total_number_of_frames {
                    if self.core_map[i].bit0(){
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

        if best_start_index_so_far == self.total_number_of_frames {
            return None;
        }

        for k in best_start_index_so_far..best_start_index_so_far + frames_requested{
            assert_eq!(self.core_map[k].bit0(), false);

            self.core_map[k].bit0() = true;
            self.core_map[k].bit3() = true;
        }

        let temp = CURR_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + frames_requested;
        CURR_NUM_FRAMES_ALLOCATED.store(temp, Ordering::Relaxed);

        return Some(best_start_index_so_far..best_start_index_so_far + frames_requested);
    }
}


