#![allow(dead_code)] // Suppress unused warnings

use crate::block::block_core::{Block, BlockSector};
use crate::system::unwrap_system_mut;
use alloc::boxed::Box;
use core::ptr::copy_nonoverlapping;
use kidneyos_shared::mem::PAGE_FRAME_SIZE;
use kidneyos_shared::sizes::{SECTOR_SIZE, SWAP_SECTORS, SWAP_SIZE};
use kidneyos_shared::{bitfield, println};

const FRAMES: usize = SWAP_SIZE as usize / PAGE_FRAME_SIZE;
const SWAP_INDEX: usize = 3;
const SECTORS_IN_PAGE: u32 = PAGE_FRAME_SIZE as u32 / SECTOR_SIZE;

// TODO: Convert VA usizes to to VirtAddr after merge.
const SWAP_START: BlockSector = 10240;

// TODO: Convert bitmap to an actual bitmap.
pub struct SwapSpace {
    bitmap: Box<[u8; FRAMES]>,
    mem_start: *mut u8,
    mem_limit: *mut u8,
}

// Bit 9 for swap bit.
impl SwapSpace {
    /// # Safety
    ///
    ///  mem_start + max_frames * PAGE_FRAME_SIZE is within bounds.
    pub unsafe fn new(mem_start: *mut u8, max_frames: usize) -> Self {
        let mem_limit = unsafe { mem_start.add(max_frames * PAGE_FRAME_SIZE) };

        Self {
            bitmap: Box::new([0; FRAMES]),
            mem_start,
            mem_limit,
        }
    }

    /// Read data into (simulated) physical memory into 'frame' from 'swap_offset'
    ///
    /// # Safety
    //
    /// Assumes <frame> is valid, and not out of bounds.
    /// We do not change the PTE bits here.
    pub unsafe fn swap_in(&mut self, swap_idx: usize, frame: usize) {
        if swap_idx >= FRAMES {
            panic!("swap_offset out of bounds!");
        }

        if self.bitmap[swap_idx] != 0 {
            panic!("Nothing exists at current swap_offset!");
        }

        let swap: &mut Block =
            unsafe { unwrap_system_mut().block_manager.by_id(SWAP_INDEX).unwrap() };

        let mut frame_ptr: *mut u8 = self.mem_start.add(frame * PAGE_FRAME_SIZE);

        let swap_sector: BlockSector = (swap_idx * PAGE_FRAME_SIZE) as u32 / SECTOR_SIZE;

        // Read from swap
        let mut buffer = [0u8; SECTOR_SIZE as usize];
        let buffer_ref: &mut [u8; SECTOR_SIZE as usize] = &mut buffer;

        for i in 0..SECTORS_IN_PAGE {
            swap.read(swap_sector + (i * SECTOR_SIZE), buffer_ref)
                .expect("Block read error");

            // Write sector to pointer
            unsafe {
                copy_nonoverlapping(buffer_ref.as_ptr(), frame_ptr, SECTOR_SIZE as usize);
            }

            // Increment pointer
            frame_ptr = frame_ptr.add(SECTOR_SIZE as usize);
        }

        self.bitmap[swap_idx] = 0;
    }

    /// Write data from (simulated) physical memory from 'frame' into 'swap_offset'
    ///
    /// # Safety
    ///
    /// Assumes <frame> is valid, and not out of bounds.
    /// We do not change the PTE bits here.
    pub unsafe fn swap_out(&mut self, swap_idx: usize, frame: usize) {
        if swap_idx >= FRAMES {
            panic!("swap_offset out of bounds!");
        }

        let swap: &mut Block =
            unsafe { unwrap_system_mut().block_manager.by_id(SWAP_INDEX).unwrap() };

        let mut frame_ptr = self.mem_start.add(frame * PAGE_FRAME_SIZE);

        let swap_sector: BlockSector = (swap_idx * PAGE_FRAME_SIZE) as u32 / SECTOR_SIZE;

        // Write to swap
        let mut buffer = [0u8; SECTOR_SIZE as usize];
        let buffer_ref: *mut u8 = buffer.as_mut_ptr();

        for i in 0..SECTORS_IN_PAGE {
            unsafe {
                copy_nonoverlapping(frame_ptr, buffer_ref, SECTOR_SIZE as usize);
            }

            swap.write(swap_sector + (i * SECTOR_SIZE), &buffer)
                .expect("Block write error");

            frame_ptr = frame_ptr.add(SECTOR_SIZE as usize);
        }

        self.bitmap[swap_idx] = 1;
    }
}
