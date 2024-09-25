use core::error::Error;
use core::fmt::{Debug, Display, Formatter};

/// Error type for block operations
pub enum BlockError {
    /// The sector is out of bounds (greater than the block size)
    SectorOutOfBounds,
    /// The buffer has an invalid size (not `BLOCK_SECTOR_SIZE`)
    BufferInvalid,
    /// Error reading from the disk
    ReadError,
    /// Error writing to the disk
    WriteError,
}

impl Debug for BlockError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            BlockError::SectorOutOfBounds => write!(f, "SectorOutOfBounds"),
            BlockError::BufferInvalid => write!(f, "BufferInvalid"),
            BlockError::ReadError => write!(f, "ReadError"),
            BlockError::WriteError => write!(f, "WriteError"),
        }
    }
}

impl Display for BlockError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for BlockError {
    fn description(&self) -> &str {
        match self {
            BlockError::SectorOutOfBounds => "Sector out of bounds (greater than the block size)",
            BlockError::BufferInvalid => "Invalid buffer size (not `BLOCK_SECTOR_SIZE`)",
            BlockError::ReadError => "Error reading from the block device",
            BlockError::WriteError => "Error writing to the block device",
        }
    }
}
