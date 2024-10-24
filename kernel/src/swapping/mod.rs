mod page_replacement;
use alloc::boxed::Box;
use kidneyos_shared::sizes::KB;

const FRAMES: usize = 3;
const SWAP_SIZE: usize = FRAMES * KB;

// TODO: Convert VA usizes to to VirtAddr after merge.

// TODO: Convert bitmap to an actual bitmap.
pub struct SwapSpace {
    area: Box<[u8; SWAP_SIZE]>,
    bitmap: Box<[u8; FRAMES]>,
}

// TODO: Is there a better way to memcpy()?
unsafe fn copy_nonoverlapping(src: *const u8, dst: *mut u8, count: usize) {
    for i in 0..count {
        *dst.add(i) = *src.add(i);
    }
}

// Is in memory currently.
impl SwapSpace {
    pub fn new() -> Self {
        {
            Self {
                area: Box::new([0; SWAP_SIZE]),
                bitmap: Box::new([0; FRAMES]),
            }
        }
    }

    // Read data into (simulated) physical memory 'frame_addr' from 'swap_offset'
    pub unsafe fn swap_in(&mut self, swap_offset: usize, frame_addr: usize) {
        if swap_offset >= FRAMES {
            panic!("swap_offset out of bounds!");
        }
        if self.bitmap[swap_offset] != 0 {
            panic!("Nothing exists at current swap_offset!");
        }

        let src = self.area.as_ptr().add(swap_offset * KB);
        let dst = frame_addr as *mut u8;

        copy_nonoverlapping(src, dst, KB);
    }

    // Write data from (simulated) physical memory 'frame_addr' to 'swap_offset'
    pub unsafe fn swap_out(&mut self, swap_offset: usize, frame_addr: usize) {
        if swap_offset >= FRAMES {
            panic!("swap_offset out of bounds!");
        }

        let dst = self.area.as_mut_ptr().add(swap_offset * KB);
        let src = frame_addr as *const u8;

        self.bitmap[swap_offset] = 1;

        copy_nonoverlapping(src, dst, KB);
    }
}
