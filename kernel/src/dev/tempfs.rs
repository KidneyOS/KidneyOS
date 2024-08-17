
use super::super::sync::irq::MutexIrq;
use super::block::{BLOCK_SECTOR_SIZE, BlockSector, BlockDriver, BlockManager};
use alloc::vec::{Vec};

pub struct TempFs {
    sects: Vec<[u8; BLOCK_SECTOR_SIZE]>, 
}
impl TempFs {
    fn new(sectors: usize) -> TempFs{
        let mut sects = Vec::with_capacity(sectors);
        for i in 0..sectors {
            sects.push([0; BLOCK_SECTOR_SIZE]);
        }
        TempFs{ sects }
    }
    pub fn read(&self, fd: TempFsDisk, sector: BlockSector, buf: &mut [u8]) {
        for i in 0..BLOCK_SECTOR_SIZE {
            buf[i] = self.sects[sector as usize][i];
        }
    }

    pub fn write(&mut self, fd: TempFsDisk, sector: BlockSector, buf: &[u8]) {
        for i in 0..BLOCK_SECTOR_SIZE {
            self.sects[sector as usize][i] = buf[i];
        }
    }
}
static tempfs0: MutexIrq<Option<TempFs>> = MutexIrq::new(Option::None);

pub type TempFsDisk = usize;

pub fn tempfs_init(all_blocks: BlockManager ) {
    let t:  &mut Option<TempFs> = &mut tempfs0.lock();    
    *t = Option::Some(TempFs::new(1024)); 


}

pub fn tempfs_read(fd: TempFsDisk, sector: BlockSector, buf: &mut [u8]) {
    // let t: &mut TempFs = &mut tempfs0.lock().unwrap();
    // t.read(sector, buf); 
}

pub fn tempfs_write(fd: TempFsDisk, sector: BlockSector, buf: &[u8]) {
    // let t: &mut TempFs = &mut tempfs0.lock();
    // t.read(sector, buf); 

}
