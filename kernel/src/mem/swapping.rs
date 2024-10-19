use super::frame_allocator;
use core::alloc::Layout;

pub struct SwapSpace {
    ptr: *mut u8,
    layout: Layout,
}

impl SwapSpace {
    fn new(mem_pointer: *mut u8, layout: Layout) -> Self {
        unsafe { Self { ptr, layout } }
    }

    unsafe fn swap_in(&mut self, offset: usize, frame: usize) {
        // Ensure that the swap space contains enough data to read
        if size > self.layout.size() {
            panic!("Swap in size exceeds swap space capacity");
        }

    }

    unsafe fn swap_out(&mut self, offset: usize, frame: usize) {
        // Ensure that the swap space is large enough
        if size > self.layout.size() {
            panic!("Swap out size exceeds swap space capacity");
        }

    }
}


