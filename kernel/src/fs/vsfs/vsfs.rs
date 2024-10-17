use alloc::{vec, vec::Vec, string::String};
use nom::Err;
use crate::block::block_core::{Block, BLOCK_SECTOR_SIZE};
use crate::vfs::{
    DirEntries, Error, FileInfo, INodeNum, INodeType, Path, RawDirEntry, Result, SimpleFileSystem,
};



#[derive(Debug, Clone, Copy)]
struct SuperBlock {
    total_inodes: u32,
    total_blocks: u32,
    inode_bitmap_block: u32,
    data_bitmap_block: u32,
    inode_table_block: u32,
    data_block_start: u32, // TODO: add magic number
}

#[derive(Debug, Clone, Copy)]
struct Inode {
    size: u32,               // File size in bytes.
    block_pointers: [u32; 12],  // Direct pointers to data blocks.
    indirect_pointer: u32,    // Pointer to an indirect block.
    permissions: u16,         // File permissions.
    uid: u16,                 // User ID.
    gid: u16,                 // Group ID.
    link_count: u16,          // Number of links to this inode.
}

struct Bitmap {
    bits: Vec<u8>,  // Each byte represents 8 blocks (1 bit per block).
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

// Define the VSFS struct that will hold the superblock, bitmaps, and data blocks
struct VSFS {
    superblock: SuperBlock,
    inode_bitmap: Bitmap,
    data_bitmap: Bitmap,
    inodes: Vec<Inode>,
    data_blocks: Vec<Vec<u8>>,  // Example representation of data blocks in memory
    block: Block,
}

impl VSFS {
    // pub fn new(mut block: Block) -> Result<Self> {

    // }
}

impl SimpleFileSystem for VSFS {
    fn root(&self) -> INodeNum {
        todo!()
    }

    fn open(&mut self, inode: INodeNum) -> Result<()> {
        todo!()
    }

    fn readdir(&mut self, dir: INodeNum) -> Result<DirEntries> {
        todo!()
    }

    fn release(&mut self, inode: INodeNum) {
        todo!()
    }

    fn read(&mut self, file: INodeNum, offset: u64, buf: &mut [u8]) -> Result<usize> {
        todo!()
    }

    fn stat(&mut self, file: INodeNum) -> Result<FileInfo> {
        todo!()
    }

    fn readlink(&mut self, link: INodeNum) -> Result<String> {
        todo!()
    }

    fn create(&mut self, parent: INodeNum, name: &Path) -> Result<INodeNum> {
        Err(Error::ReadOnlyFS)
    }

    fn mkdir(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }

    fn unlink(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }

    fn rmdir(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }

    fn write(&mut self, file: INodeNum, offset: u64, buf: &[u8]) -> Result<usize> {
        Err(Error::ReadOnlyFS)
    }

    fn link(&mut self, source: INodeNum, parent: INodeNum, name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }

    fn symlink(&mut self, link: &Path, parent: INodeNum, name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }

    fn truncate(&mut self, file: INodeNum, size: u64) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }

}