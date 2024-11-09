use core::alloc::AllocError;
use core::ptr::NonNull;
use kidneyos_shared::mem::PAGE_FRAME_SIZE;

pub struct DummyAllocatorSolution {
    start_address: usize,
    end_address: usize,
}

impl DummyAllocatorSolution {
    pub const fn new_in(start_address: usize, end_address: usize) -> Self {
        DummyAllocatorSolution {
            start_address,
            end_address,
        }
    }
    
    /// Allocate a region of memory equals to "total_frames" * PAGE_FRAME_SIZE bytes for 
    /// the coremap
    /// 
    /// This allocation should do two things:
    /// 
    /// 1. Returns a piece of memory starting at "start_address" used to store the 
    /// CoreMap Entries for actual frames
    /// 
    /// 2. Increment "start_address" to point to the next free frame in memory
    ///
    /// The region of memory allocated to the coremap should never be freed; it should 
    /// remain there until the kernel stops running
    pub fn alloc(&mut self, total_frames: usize) -> Result<NonNull<[u8]>, AllocError> {
        // Don't think this will ever happen, but good to have a check for it
        if self.start_address + (PAGE_FRAME_SIZE * total_frames) > self.end_address {
            return Err(AllocError);
        }

        let new_addr = (self.start_address + (PAGE_FRAME_SIZE * total_frames))
            .next_multiple_of(PAGE_FRAME_SIZE);

        let ret = Ok(NonNull::slice_from_raw_parts(
            NonNull::new(self.start_address as *mut u8).ok_or(AllocError)?,
            total_frames * PAGE_FRAME_SIZE,
        ));

        self.start_address = new_addr;
        ret
    }

    pub fn get_start_address(&self) -> usize {
        self.start_address
    }

    pub fn get_end_address(&self) -> usize {
        self.end_address
    }

    pub fn set_start_address(&mut self, new_start: usize) {
        self.start_address = new_start;
    }

    pub fn set_end_address(&mut self, new_end: usize) {
        self.end_address = new_end;
    }
}
