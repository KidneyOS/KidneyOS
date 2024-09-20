#![allow(dead_code)] // Suppress unused warnings

use crate::drivers::dummy_device::DummyDevice;
use alloc::{string::String, vec::Vec};
use core::fmt;
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
    Kernel(BlockSector),
    /// File system
    FileSystem(BlockSector),
    /// Scratch
    Scratch(BlockSector),
    /// Swap
    Swap(BlockSector),
    /// "Raw" device with unidentified contents
    Raw(BlockSector),
    /// Owned by non-KidneyOS operating system
    Foreign(BlockSector),
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlockType::Kernel(_) => write!(f, "Kernel"),
            BlockType::FileSystem(_) => write!(f, "File System"),
            BlockType::Scratch(_) => write!(f, "Scratch"),
            BlockType::Swap(_) => write!(f, "Swap"),
            BlockType::Raw(_) => write!(f, "Raw"),
            BlockType::Foreign(_) => write!(f, "Foreign"),
        }
    }
}

/// Lower-level interface to block device drivers
pub trait BlockOp {
    /// Read a block sector
    unsafe fn read(&self, sector: BlockSector, buf: &mut [u8]);
    /// Write a block sector
    unsafe fn write(&self, sector: BlockSector, buf: &[u8]);
}

/// Supported block drivers
#[derive(Clone, Copy, PartialEq)]
pub enum BlockDriver {
    // TODO: Add drivers here
    Dummy(DummyDevice),
}

impl BlockDriver {
    /// Unwrap the block driver to get the underlying block operation
    fn unwrap(&self) -> &dyn BlockOp {
        match self {
            // TODO: Add drivers here
            BlockDriver::Dummy(driver) => driver,
        }
    }

    /// Read a block sector
    fn read(&self, sector: BlockSector, buf: &mut [u8]) {
        unsafe {
            self.unwrap().read(sector, buf);
        }
    }

    /// Write a block sector
    fn write(&self, sector: BlockSector, buf: &[u8]) {
        unsafe {
            self.unwrap().write(sector, buf);
        }
    }
}

/// A block device
///
/// **Note:** Once blocks are made they are immutable
#[derive(PartialEq, Clone)]
pub struct Block {
    /// Unique and immutable index of the block
    index: usize,
    /// Tha name of the block device
    block_name: String,

    /// The type of block
    block_type: BlockType,
    /// The block driver
    driver: BlockDriver,

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
    /// Panics if the buffer is not the correct size (i.e., `BLOCK_SECTOR_SIZE` bytes).
    fn verify_buffer(buf: &[u8]) {
        if buf.len() != BLOCK_SECTOR_SIZE {
            panic!("Invalid buffer size {}", buf.len());
        }
    }

    /// Verifies that `sector` is a valid offset within the block device.
    ///
    /// Panics if the sector is out of bounds.
    fn check_sector(&self, sector: BlockSector) {
        if sector >= self.block_size {
            panic!(
                "{}: Invalid sector {} (block size: {})",
                self.block_name, sector, self.block_size
            );
        }
    }

    /// Reads sector `sector` from the block device into `buf`, which must have room for
    /// `BLOCK_SECTOR_SIZE` bytes.
    pub fn read(&mut self, sector: BlockSector, buf: &mut [u8]) {
        self.check_sector(sector);
        Self::verify_buffer(buf);

        self.driver.read(sector, buf);
        self.read_count += 1;
    }

    /// Writes sector `sector` from `buf`, which must contain `BLOCK_SECTOR_SIZE` bytes. Returns
    /// after the block device has acknowledged receiving the data.
    pub fn write(&mut self, sector: BlockSector, buf: &[u8]) {
        self.check_sector(sector);
        Self::verify_buffer(buf);

        // Ensure that we are not writing to a foreign block
        assert!(
            self.block_type != BlockType::Foreign(0),
            "Cannot write to foreign block"
        );

        self.driver.write(sector, buf);
        self.write_count += 1;
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
        driver: BlockDriver,
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
        if idx >= self.all_blocks.len() {
            return None;
        }

        Some(&mut self.all_blocks[idx])
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
    // BlockManager::new()

    // TODO: remove this dummy block device when real block devices are implemented
    let mut block_manager = BlockManager::new();

    // Register a dummy block device
    block_manager.register_block(
        BlockType::Raw(0),
        "Dummy",
        4,
        BlockDriver::Dummy(DummyDevice::new()),
    );

    block_manager
}
