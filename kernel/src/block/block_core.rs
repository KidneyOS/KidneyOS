#![allow(dead_code)] // Suppress unused warnings

use crate::block::block_error::BlockError;
use crate::interrupts::{intr_get_level, IntrLevel};
use alloc::boxed::Box;
use alloc::{string::String, vec::Vec};
use core::fmt;
use core::result::Result;
use kidneyos_shared::println;

/// Size of a block device in bytes.
///
/// All IDE disks use this sector size, as do most USB and SCSI disks.
pub const BLOCK_SECTOR_SIZE: usize = 512;

/// Index of a block device sector.
///
/// Good enough for devices up to 2 TB.
pub type BlockSector = u32;

/// Types of blocks
#[derive(PartialEq, Copy, Clone)]
pub enum BlockType {
    /// OS Kernel
    Kernel,
    /// File system
    FileSystem,
    /// Scratch
    Scratch,
    /// Swap
    Swap,
    /// "Raw" device with unidentified contents
    Raw,
    /// Owned by non-KidneyOS operating system
    Foreign,
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlockType::Kernel => write!(f, "Kernel"),
            BlockType::FileSystem => write!(f, "File System"),
            BlockType::Scratch => write!(f, "Scratch"),
            BlockType::Swap => write!(f, "Swap"),
            BlockType::Raw => write!(f, "Raw"),
            BlockType::Foreign => write!(f, "Foreign"),
        }
    }
}

/// Lower-level interface to block device drivers
pub trait BlockOp {
    /// Read a block sector
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled. Otherwise, the block device may not
    /// wake up after the read operation is complete.
    unsafe fn read(&mut self, sector: BlockSector, buf: &mut [u8]) -> Result<(), BlockError>;

    /// Write a block sector
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled. Otherwise, the block device may not
    /// wake up after the write operation is complete.
    unsafe fn write(&mut self, sector: BlockSector, buf: &[u8]) -> Result<(), BlockError>;
}

/// A block device
///
/// **Note:** Once blocks are made they are immutable
pub struct Block {
    /// Unique and immutable index of the block
    index: usize,
    /// Tha name of the block device
    block_name: String,

    /// The type of block
    block_type: BlockType,
    /// The block driver
    driver: Box<dyn BlockOp>,

    /// The size of the block device in sectors
    block_size: BlockSector,

    /// The read count
    read_count: u32,
    /// The write count
    write_count: u32,
}

impl Block {
    /// Verifies that `buf` is a valid buffer for reading or writing a block sector.
    ///
    /// Returns `true` if the buffer is valid, `false` otherwise.
    fn is_buffer_valid(buf: &[u8]) -> bool {
        buf.len() == BLOCK_SECTOR_SIZE
    }

    /// Verifies that `sector` is a valid offset within the block device.
    ///
    /// Returns `true` if the sector is valid, `false` otherwise.
    fn is_sector_valid(&self, sector: BlockSector) -> bool {
        sector < self.block_size
    }

    /// Reads sector `sector` from the block device into `buf`, which must have room for
    /// `BLOCK_SECTOR_SIZE` bytes.
    ///
    /// Panics if interrupts are disabled.
    pub fn read(&mut self, sector: BlockSector, buf: &mut [u8]) -> Result<(), BlockError> {
        assert_eq!(
            intr_get_level(),
            IntrLevel::IntrOn,
            "Block::read must not be called with interrupts disabled."
        );
        if !self.is_sector_valid(sector) {
            return Err(BlockError::SectorOutOfBounds);
        }
        if !Self::is_buffer_valid(buf) {
            return Err(BlockError::BufferInvalid);
        }

        self.read_count += 1;
        unsafe { self.driver.read(sector, buf) }
    }

    /// Writes sector `sector` from `buf`, which must contain `BLOCK_SECTOR_SIZE` bytes. Returns
    /// after the block device has acknowledged receiving the data.
    ///
    /// Panics if interrupts are disabled.
    pub fn write(&mut self, sector: BlockSector, buf: &[u8]) -> Result<(), BlockError> {
        assert_eq!(
            intr_get_level(),
            IntrLevel::IntrOn,
            "Block::write must not be called with interrupts disabled."
        );
        if !self.is_sector_valid(sector) {
            return Err(BlockError::SectorOutOfBounds);
        }
        if !Self::is_buffer_valid(buf) {
            return Err(BlockError::BufferInvalid);
        }

        // Ensure that we are not writing to a foreign block
        assert!(
            self.block_type != BlockType::Foreign,
            "Cannot write to foreign block"
        );

        self.write_count += 1;
        unsafe { self.driver.write(sector, buf) }
    }

    // Block getters -----------------------------------------------------------

    pub fn get_type(&self) -> BlockType {
        self.block_type
    }
    pub fn get_size(&self) -> BlockSector {
        self.block_size
    }
    pub fn get_name(&self) -> &str {
        &self.block_name
    }
    pub fn get_index(&self) -> usize {
        self.index
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "    {:04} | \"{}\" ({}): {:04} sectors, {:04} read, {:04} write",
            self.index,
            self.block_name,
            self.block_type,
            self.block_size,
            self.read_count,
            self.write_count
        )
    }
}

/// Maintain a list of blocks
pub struct BlockManager {
    /// All the block devices
    all_blocks: Vec<Block>,
    /// The maximum index of the block devices
    max_index: usize,
}

impl BlockManager {
    /// Create a new block manager
    fn new() -> Self {
        BlockManager::with_capacity(10)
    }

    /// Create a new block manager with a given capacity
    fn with_capacity(cap: usize) -> Self {
        BlockManager {
            all_blocks: Vec::with_capacity(cap),
            max_index: 0,
        }
    }

    /// Register a block device with the given `name`. The block device's `size` in sectors and its
    /// `device_type` must be prvided, as well as the `driver` to access the block.
    ///
    /// Returns the index of the block device.
    pub fn register_block(
        &mut self,
        block_type: BlockType,
        block_name: &str,
        block_size: BlockSector,
        driver: Box<dyn BlockOp>,
    ) -> usize {
        self.all_blocks.push(Block {
            index: self.max_index,
            block_name: String::from(block_name),
            block_type,
            driver,
            block_size,
            read_count: 0,
            write_count: 0,
        });

        println!(
            "Registered block device \"{}\" ({} type) with {} sectors",
            self.all_blocks[self.max_index].block_name, block_type, block_size,
        );

        self.max_index += 1;
        self.max_index - 1
    }

    /// Get the block device with the given `index`.
    ///
    /// If the index is out of bounds, returns `None`.
    pub fn by_id(&mut self, idx: usize) -> Option<&mut Block> {
        self.all_blocks.get_mut(idx)
    }

    /// Get the block device with the given `name`.
    ///
    /// If the name is not found, returns `None`.
    ///
    /// **Note:** This function is very inefficient and should be avoided.
    pub fn by_name(&mut self, name: &str) -> Option<&mut Block> {
        self.all_blocks.iter_mut().find(|b| b.block_name == name)
    }
}

impl fmt::Display for BlockManager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Block Devices:")?;
        for block in self.all_blocks.iter() {
            writeln!(f, "{}", block)?;
        }
        Ok(())
    }
}

/// Initialize the block layer
pub fn block_init() -> BlockManager {
    BlockManager::new()
}
