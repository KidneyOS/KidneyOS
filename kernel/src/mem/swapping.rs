use crate::load_arguments;

// use super::frame_allocator;
use core::alloc::Layout;

pub struct SwapSpace {
    ptr: *mut u8,
    layout: Layout,
}

impl SwapSpace {
    fn new(mem_ptr: *mut u8, layout: Layout) -> Self {
        unsafe { Self { ptr:mem_ptr, layout } }
    }
    

    unsafe fn swap_in(&mut self, offset: usize, frame: usize) {
    }

    unsafe fn swap_out(&mut self, offset: usize, frame: usize) {
    }
}


