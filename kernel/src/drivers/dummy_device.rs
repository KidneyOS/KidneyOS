use crate::block::block_core::{BlockOp, BlockSector};

/// A dummy block device driver that does nothing but panicking, should not be used in production.
#[derive(Clone, Copy, PartialEq)]
pub struct DummyDevice;

impl DummyDevice {
    pub const fn new() -> Self {
        Self
    }
}

impl BlockOp for DummyDevice {
    unsafe fn read(&self, sector: BlockSector, _buf: &mut [u8]) {
        panic!("Reading dummy device at sector {}", sector);
    }
    unsafe fn write(&self, sector: BlockSector, _buf: &[u8]) {
        panic!("Writing dummy device at sector {}", sector);
    }
}
