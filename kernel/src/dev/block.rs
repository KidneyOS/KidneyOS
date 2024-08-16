use alloc::{vec::Vec, string::String};
use super::super::sync::irq::MutexIrq;
use super::ide::{ATADisk, ide_read, ide_write};
use super::tempfs::{TempFsDisk, tempfs_read, tempfs_write};

pub const BLOCK_SECTOR_SIZE: usize = 512;
pub type BlockSector = u32;

// partiton types have offset as part of the enum
#[derive(PartialEq, Copy, Clone)]
pub enum BlockType {
    BlockKernel(BlockSector),
    BlockFilesys(BlockSector),
    BlockScratch(BlockSector),
    BlockSwap(BlockSector),
    BlockForeign(BlockSector),
    BlockRaw,
}

#[derive(PartialEq, Copy, Clone)]
pub enum BlockDriver {
    ATAPio(ATADisk),
    TempFs(TempFsDisk)
}

impl BlockDriver {
    fn read(&self, sector: BlockSector, buf: &mut [u8]) {
        match self {
            BlockDriver::ATAPio(d) => ide_read(*d, sector, buf),
            BlockDriver::TempFs(d) => tempfs_read(*d, sector, buf), 
        }

    }
    fn write(&self, sector: BlockSector, buf: &[u8]) {
        match self {
            BlockDriver::ATAPio(d) => ide_write(*d, sector, buf),
            BlockDriver::TempFs(d) => tempfs_write(*d, sector, buf), 
        }
    }

}

#[derive(PartialEq, Clone)]
pub struct Block {
    driver: BlockDriver,
    block_type: BlockType,
    block_name: String,
    block_size: BlockSector,
    idx: usize,
}


impl Block {
    pub fn block_read(&self, sector: BlockSector, buf: &mut [u8]){
        self.driver.read(sector, buf);
    }
    pub fn block_write(&self, sector: BlockSector, buf: &[u8]){
        self.driver.write(sector, buf);
    }
    pub fn block_type(&self) -> BlockType{
        self.block_type
    }
    pub fn block_size(&self) -> BlockSector {
        self.block_size
    }
    pub fn block_name(&self) -> &str {
        &self.block_name
    }
    // To be used by block manager only
    fn block_set_idx(&mut self, idx: usize){
        self.idx = idx
    }
    fn block_idx(&self) -> usize{
        self.idx
    }

    pub fn driver(&self) -> BlockDriver {
        self.driver
    }
}


//maintain a list of blocks
pub struct BlockManager {
    all_blocks: Vec<Block>,
}

impl BlockManager {

    fn new() -> Self {
        BlockManager::with_capacity(10)
    }

    fn with_capacity(cap: usize) -> Self {
        let mut all_blocks: Vec<Block> = Vec::with_capacity(cap);
        BlockManager {
            all_blocks 
        }
    }

    pub fn block_register(&mut self, block_type: BlockType, block_name: String, block_size: BlockSector, block_driver: BlockDriver) -> usize{
        let idx = self.all_blocks.len();
        self.all_blocks.push( Block {
            driver: block_driver,
            block_type,
            block_name: block_name.into(),
            block_size,
            idx,
        });
        idx
    } 

    pub fn by_id(&self, idx: usize) -> Block {
       self.all_blocks[idx].clone() 
    }



    pub fn blocks_by_type() {

    }
    pub fn block_by_name() {

    }


}

pub fn block_init() -> BlockManager {
    BlockManager::new()
}
