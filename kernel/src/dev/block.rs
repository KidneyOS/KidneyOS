


pub const BLOCK_SECTOR_SIZE:usize =  512;
pub type BlockSector = u32;

pub enum BlockType{
    BlockKernel,
    // BlockFilesys,
    // BlockScratch,
    // BlockSwap,
    // BlockRaw,
    // BlockForeign,
}


pub trait BlockDevice{
    fn block_read(&self, sector: BlockSector, buf: &mut [u8]);
    fn block_write(&self, sector: BlockSector, buf: &[u8]);
    fn get_block_type(&self) -> BlockType;
}





