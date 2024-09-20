use crate::block::block_core::{BlockError, BlockOp, BlockSector};

/// A dummy block device driver that does nothing but panicking, should not be used in production.
#[derive(Clone, Copy, PartialEq)]
pub struct DummyDevice;

impl DummyDevice {
    pub const fn new() -> Self {
        Self
    }
}

impl BlockOp for DummyDevice {
    unsafe fn read(&mut self, sector: BlockSector, _buf: &mut [u8]) -> Result<(), BlockError> {
        panic!("Reading dummy device at sector {}", sector);
    }
    unsafe fn write(&mut self, sector: BlockSector, _buf: &[u8]) -> Result<(), BlockError> {
        panic!("Writing dummy device at sector {}", sector);
    }
}
