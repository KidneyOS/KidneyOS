use alloc::{boxed::Box, vec::Vec};

pub const BLOCK_SECTOR_SIZE: usize = 512;
pub type BlockSector = u32;


#[derive(PartialEq, Copy, Clone)]
pub enum BlockType {
    BlockKernel,
    BlockFilesys,
    BlockScratch,
    BlockSwap,
    BlockRaw,
    BlockForeign,
}

pub trait BlockDevice {
    fn block_read(&self, sector: BlockSector, buf: &mut [u8]);
    fn block_write(&self, sector: BlockSector, buf: &[u8]);
    fn block_type(&self) -> BlockType;
    fn block_size(&self) -> usize;
    fn block_name(&self) -> &str;
    // To be used by block manager only
    fn block_set_idx(&mut self, idx: usize); //stores idx
    fn block_idx(&self) -> usize; //retrieves idx
}


//maintain a list of blocks
pub struct BlockManager {
    all_blocks: Vec<Box<dyn BlockDevice>>,
}

impl BlockManager {

    fn new() -> Self {
        BlockManager::with_capacity(10)
    }

    fn with_capacity(cap: usize) -> Self {
        let mut all_blocks: Vec<Box<dyn BlockDevice>> = Vec::with_capacity(cap);
        BlockManager {
            all_blocks 
        }
    }

    pub fn register_block(&mut self, mut block: Box<dyn BlockDevice>) {
        block.block_set_idx(self.all_blocks.len() + 1);
        self.all_blocks.push(block);
    } 


    pub fn blocks_by_type() {

    }
    pub fn block_by_name() {

    }




}

pub fn block_init() -> BlockManager {
    BlockManager::new()
}
