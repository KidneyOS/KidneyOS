use alloc::vec::Vec;
use alloc::vec;

#[derive(Debug, Clone, Copy)]
pub struct SuperBlock {
    pub magic_number: u64,  // Must match VSFS_MAGIC
    pub fs_size: u64,  // File system size in bytes.
    pub num_inodes: u32,  // Total number of inodes (set by mkfs).
    pub free_inodes: u32,  // Number of available inodes.
    pub num_blocks: u32,  // File system size in blocks.
    pub free_blocks: u32,  // Number of available blocks.
    pub data_start: u32,  // First block after inode table.
}

pub struct Bitmap {
    pub bits: Vec<u8>,  // Each byte represents 8 blocks (1 bit per block).
}

impl Bitmap {
    pub fn new(num_bits: u32) -> Self {
        let num_bytes = (num_bits + 7) / 8;  // Calculate how many bytes are needed.
        Self { bits: vec![0; num_bytes as usize] }
    }

    pub fn is_allocated(&self, index: u32) -> bool {
        let byte_index = (index / 8) as usize;
        let bit_offset = (index % 8) as u8;
        self.bits[byte_index] & (1 << bit_offset) != 0
    }

    pub fn allocate(&mut self, index: u32) {
        let byte_index = (index / 8) as usize;
        let bit_offset = (index % 8) as u8;
        self.bits[byte_index] |= 1 << bit_offset;
    }

    pub fn deallocate(&mut self, index: u32) {
        let byte_index = (index / 8) as usize;
        let bit_offset = (index % 8) as u8;
        self.bits[byte_index] &= !(1 << bit_offset);
    }
}



