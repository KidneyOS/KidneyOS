use alloc::{vec::Vec, sync::Arc, string::String};
use super::ide::ATADisk;
use super::tempfs::TempFsDisk;


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
    BlockTempfs,
    BlockRaw,
}

impl BlockType {
    fn get_offset(&self) -> BlockSector {
        match self {
            BlockType::BlockKernel(o) => *o,
            BlockType::BlockFilesys(o) => *o,
            BlockType::BlockScratch(o) => *o,
            BlockType::BlockSwap(o) => *o,
            BlockType::BlockForeign(o) => *o,
            _ => 0,
        } 
    }
}

pub trait BlockOperations {
    unsafe fn read(&self, sector: BlockSector, buf: &mut [u8]) -> u8; 
    unsafe fn write(&self, sector: BlockSector, buf: &[u8]) -> u8;
}

#[derive(PartialEq, Copy, Clone)]
pub enum BlockDriver {
    ATAPio(ATADisk),
    TempFs(TempFsDisk),
    // FUSE(Arc<dyn FuseDriver>),
}


impl BlockDriver {
    fn unwrap(&self) -> &impl BlockOperations {
        match self {
            BlockDriver::ATAPio(d) => d,
            BlockDriver::TempFs(d) => d, 
        }
    }
    unsafe fn read(&self, sector: BlockSector, buf: &mut [u8]) -> u8{
        let ops: &dyn BlockOperations = self.unwrap();
        ops.read(sector, buf)
    }
    unsafe fn write(&self, sector: BlockSector, buf: &[u8]) -> u8 {
        let ops: &dyn BlockOperations = self.unwrap();
        ops.write(sector, buf)
    }

}

// once blocks are made they are immutable
#[derive(PartialEq, Clone)]
pub struct Block {
    driver: BlockDriver,
    block_type: BlockType,
    block_name: String,
    block_size: BlockSector,
    idx: usize,
}

impl Block {
    pub fn block_read(&self, sector: BlockSector, buf: &mut [u8]) -> u8{
        let offset = self.block_type.get_offset();  
        if sector + offset > self.block_size() || buf.len() < BLOCK_SECTOR_SIZE {
            return 1
        }
        unsafe {
            self.driver.read(sector + offset, buf)
        }
    }
    pub fn block_write(&self, sector: BlockSector, buf: &[u8]) -> u8{
        let offset = self.block_type.get_offset();  
        if sector + offset > self.block_size() || buf.len() < BLOCK_SECTOR_SIZE {
            return 1
        }
        unsafe {
            self.driver.write(sector + offset, buf)
        }
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
        BlockManager {
            all_blocks: Vec::with_capacity(cap)
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
    //TODO: rest of fs code should take reference to block instead of cloning
    pub fn by_id(&self, idx: usize) ->  Block {
       self.all_blocks[idx].clone() 
    }

    //TODO: rest of fs code should take reference to block instead of cloning
    pub fn by_name(&self, name: &str) -> Option<Block>{
        for i in self.all_blocks.iter() {
            if i.block_name == name {
                return Option::Some(i);
            }
        }
        Option::None
    }


}

pub fn block_init() -> BlockManager {
    BlockManager::new()
}
