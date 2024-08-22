use super::super::sync::irq::MutexIrq;
use super::block::{
    BlockDriver, BlockManager, BlockOperations, BlockSector, BlockType, BLOCK_SECTOR_SIZE,
};
use alloc::vec::Vec;

pub struct TempFs {
    sects: Vec<[u8; BLOCK_SECTOR_SIZE]>,
}
impl TempFs {
    fn new(sectors: usize) -> TempFs {
        let mut sects = Vec::with_capacity(sectors);

        for _ in 0..sectors {
            sects.push([0; BLOCK_SECTOR_SIZE]);
        }
        TempFs { sects }
    }
    pub fn read(&self, sector: BlockSector, buf: &mut [u8]) {
        buf[..BLOCK_SECTOR_SIZE].copy_from_slice(&self.sects[sector as usize][..BLOCK_SECTOR_SIZE]);
    }

    pub fn write(&mut self, sector: BlockSector, buf: &[u8]) {
        self.sects[sector as usize][..BLOCK_SECTOR_SIZE].copy_from_slice(&buf[..BLOCK_SECTOR_SIZE]);
    }
}
static TEMPFS0: MutexIrq<Option<TempFs>> = MutexIrq::new(Option::None);

// tempfs disk descriptor type
#[derive(Copy, Clone, PartialEq)]
pub struct TempFsDisk;

pub fn tempfs_init(mut all_blocks: BlockManager) {
    let t: &mut Option<TempFs> = &mut TEMPFS0.lock();
    *t = Option::Some(TempFs::new(1024));
    all_blocks.block_register(
        BlockType::Tempfs,
        "tempfs0".into(),
        1024 as BlockSector,
        BlockDriver::TempFs(TempFsDisk),
    );
}

impl BlockOperations for TempFsDisk {
    unsafe fn read(&self, sector: BlockSector, buf: &mut [u8]) -> u8 {
        let t: &mut Option<TempFs> = &mut TEMPFS0.lock();
        t.as_mut().unwrap().read(sector, buf);
        0
    }

    unsafe fn write(&self, sector: BlockSector, buf: &[u8]) -> u8 {
        let t: &mut Option<TempFs> = &mut TEMPFS0.lock();
        t.as_mut().unwrap().write(sector, buf);
        0
    }
}
