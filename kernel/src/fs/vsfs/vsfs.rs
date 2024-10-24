use alloc::{vec, vec::Vec, string::String};
use nom::Err;
use crate::block::block_core::{Block, BLOCK_SECTOR_SIZE};
use crate::vfs::{
    DirEntries, Error, FileInfo, INodeNum, INodeType, Path, RawDirEntry, Result, SimpleFileSystem,
};

pub const VSFS_BLOCK_SIZE: usize = 4096;  // same block size in bytes as the vsfs disk images provided
pub const BLOCK_SIZE_RATIO: usize = VSFS_BLOCK_SIZE / BLOCK_SECTOR_SIZE; // assume that the block size is a multiple of the sector size
pub const VSFS_MAGIC: u64 = 0xC5C369A4C5C369A4;  // same magic number from the vsfs disk images
pub const VSFS_DIRECT_BLOCKS: usize = 5;  // same number of direct blocks as the vsfs disk images

/* vsfs has simple layout 
 *   Block 0: superblock
 *   Block 1: inode bitmap
 *   Block 2: data bitmap
 *   Block 3: start of inode table
 *   First data block after inode table
 */
pub const VSFS_SUPERBLOCK_BLOCK: u32 = 0;
pub const VSFS_INODE_BITMAP_BLOCK: u32 = 1;
pub const VSFS_DATA_BITMAP_BLOCK: u32 = 2;
pub const VSFS_INODE_TABLE_BLOCK: u32 = 3;


#[derive(Debug, Clone, Copy)]
struct Timespec {
    tv_sec: i64,    // seconds since the Epoch
    tv_nsec: i32,   // nanoseconds
}


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

#[derive(Debug, Clone, Copy)]
struct Inode {
    mode: u32,  // File type and permissions.
    n_links: u32,  // Number of hard links.
    block_count: u32,  // Number of blocks in the file.
    size: u64,  // File size in bytes.
    mtime: Timespec,  // Last modification time.
    direct_blocks: [u32; VSFS_DIRECT_BLOCKS],  // Direct block pointers.
    indirect_block: u32,  // Indirect block pointer.
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

// Define the VSFS struct that will hold the superblock, bitmaps, and data blocks
pub struct VSFS {
    pub superblock: SuperBlock,
    pub inode_bitmap: Bitmap,
    pub data_bitmap: Bitmap,
    inodes: Vec<Inode>,
    pub data_blocks: Vec<Vec<u8>>,  // Example representation of data blocks in memory
    block: Block,
    root_inode: INodeNum,
}

impl VSFS {
    pub fn new(mut block: Block) -> Result<Self> {
        // Read the superblock from the first block
        let mut superblock = SuperBlock {
            magic_number: 0,
            fs_size: 0,
            num_inodes: 0,
            free_inodes: 0,
            num_blocks: 0,
            free_blocks: 0,
            data_start: 0,
        };

        let mut first_sector = [0; 512];
        block.read(0, &mut first_sector)?;

        // Parse the superblock from the first sector
        superblock.magic_number = u64::from_le_bytes(first_sector[0..8].try_into().unwrap());

        // Check if the magic number matches
        if superblock.magic_number != VSFS_MAGIC {
            return Err(Error::Unsupported);
        } 

        superblock.fs_size = u64::from_le_bytes(first_sector[8..16].try_into().unwrap());
        superblock.num_inodes = u32::from_le_bytes(first_sector[16..20].try_into().unwrap());
        superblock.free_inodes = u32::from_le_bytes(first_sector[20..24].try_into().unwrap());
        superblock.num_blocks = u32::from_le_bytes(first_sector[24..28].try_into().unwrap());
        superblock.free_blocks = u32::from_le_bytes(first_sector[28..32].try_into().unwrap());
        superblock.data_start = u32::from_le_bytes(first_sector[32..36].try_into().unwrap());

        // Read the inode table
        let mut inodes = vec![Inode {
            mode: 0,
            n_links: 0,
            block_count: 0,
            size: 0,
            mtime: Timespec { tv_sec: 0, tv_nsec: 0 },
            direct_blocks: [0; VSFS_DIRECT_BLOCKS],
            indirect_block: 0,
        }; superblock.num_inodes as usize];


        // Read the data blocks
        let mut data_blocks = Vec::new();

        for i in superblock.data_start..superblock.num_blocks {
            let mut data = vec![0; VSFS_BLOCK_SIZE as usize];
            for j in 0..BLOCK_SIZE_RATIO {
                block.read(j as u32 + i * BLOCK_SIZE_RATIO as u32, &mut data[(j * BLOCK_SECTOR_SIZE)..(j * BLOCK_SECTOR_SIZE + BLOCK_SECTOR_SIZE)])?;
            }
            data_blocks.push(data);
        }

        // Read the inode bitmap
        let mut inode_bitmap = Bitmap::new(superblock.num_inodes);
        let mut bits = vec![0 as u8; VSFS_BLOCK_SIZE];
        for i in 0..BLOCK_SIZE_RATIO {
            block.read((i + (VSFS_INODE_BITMAP_BLOCK as usize * BLOCK_SIZE_RATIO)) as u32, &mut bits[(i * BLOCK_SECTOR_SIZE)..(i * BLOCK_SECTOR_SIZE + BLOCK_SECTOR_SIZE)])?;
        }
        inode_bitmap.bits = bits;

        // Read the data bitmap
        let mut data_bitmap = Bitmap::new(superblock.num_blocks);
        let mut bits = vec![0 as u8; VSFS_BLOCK_SIZE];
        for i in 0..BLOCK_SIZE_RATIO {
            block.read((i + (VSFS_DATA_BITMAP_BLOCK as usize * BLOCK_SIZE_RATIO)) as u32, &mut bits[(i * BLOCK_SECTOR_SIZE)..(i * BLOCK_SECTOR_SIZE + BLOCK_SECTOR_SIZE)])?;
        }
        data_bitmap.bits = bits;

        // Create the root inode (TODO?)
        let root_inode = 0;
        
        Ok(Self {
            superblock,
            inode_bitmap,
            data_bitmap,
            inodes,
            data_blocks,
            block,
            root_inode,
        })
        
    }
}

impl SimpleFileSystem for VSFS {
    fn root(&self) -> INodeNum {
        self.root_inode
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