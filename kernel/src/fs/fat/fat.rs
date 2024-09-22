use crate::block::block_core::{Block, BLOCK_SECTOR_SIZE};
use crate::fs::fat::{error, FatType};
use crate::vfs::Result;
use zerocopy::AsBytes;

/// File Allocation Table
///
/// Lists the clusters which are allocated or free,
/// and maintains linked lists of clusters for files.
pub struct Fat {
    r#type: FatType,
    data: Vec<u32>,
}

#[derive(Clone, Copy)]
pub enum FatEntry {
    Free,
    Eof,
    Defective,
    HasNext(u32),
}

impl FatEntry {
    pub fn is_allocated(self) -> bool {
        matches!(self, FatEntry::Eof | FatEntry::HasNext(_))
    }
}

impl core::fmt::Debug for Fat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<FAT type={:?} length={}>", self.r#type, self.data.len())
    }
}

impl Fat {
    pub fn new(
        device: &mut Block,
        cluster_count: u32,
        r#type: FatType,
        sectors: core::ops::Range<u32>,
    ) -> Result<Self> {
        let mut data =
            vec![0; ((sectors.end - sectors.start) * (BLOCK_SECTOR_SIZE as u32 / 4)) as usize];
        for (i, sector) in sectors.enumerate() {
            device.read(
                sector,
                data[i * (BLOCK_SECTOR_SIZE / 4)..(i + 1) * (BLOCK_SECTOR_SIZE / 4)].as_bytes_mut(),
            )?;
        }
        let max_fat_count = data.len() as u32 * if r#type == FatType::Fat16 { 2 } else { 1 };
        if max_fat_count < cluster_count {
            return error!("FAT size is too small");
        }
        let fat = Self { data, r#type };
        // the first two FAT entries are reserved
        for i in 2..cluster_count {
            if let FatEntry::HasNext(n) = fat.entry(i) {
                if n >= cluster_count {
                    return error!("invalid entry in FAT: {n} (cluster count = {cluster_count})");
                }
            }
        }
        Ok(fat)
    }
    pub fn entry(&self, i: u32) -> FatEntry {
        match self.r#type {
            FatType::Fat16 => {
                let first_half = if cfg!(target_endian = "little") { 0 } else { 1 };
                let raw_entry = if i % 2 == first_half {
                    self.data[i as usize / 2] as u16
                } else {
                    (self.data[i as usize / 2] >> 16) as u16
                };
                match raw_entry {
                    0 => FatEntry::Free,
                    0xFFF7 => FatEntry::Defective,
                    u16::MAX => FatEntry::Eof,
                    x => FatEntry::HasNext(x.into()),
                }
            }
            FatType::Fat32 => match self.data[i as usize] {
                0 => FatEntry::Free,
                0xFFFF_FFF7 => FatEntry::Defective,
                u32::MAX => FatEntry::Eof,
                x => FatEntry::HasNext(x),
            },
        }
    }
    #[allow(dead_code)] // TODO : delete me
    pub fn flush(&mut self, _device: &mut Block) -> Result<()> {
        todo!()
    }
}
