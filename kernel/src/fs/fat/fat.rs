use crate::block::block_core::{Block, BLOCK_SECTOR_SIZE};
use crate::fs::fat::{error, free_set::FreeSet, FatType};
use crate::vfs::{Error, Result};
use alloc::{vec, vec::Vec};
use zerocopy::AsBytes;

/// File Allocation Table
///
/// Lists the clusters which are allocated or free,
/// and maintains linked lists of clusters for files.
pub struct Fat {
    r#type: FatType,
    data: Vec<u32>,
    free_clusters: FreeSet,
    /// Index of first disk sector in the first FAT
    first_fat_sector: u32,
    /// Number of disk sectors in each FAT
    sectors_per_fat: u32,
    /// Number of mirrors of this FAT on disk
    mirror_count: u32,
}

impl core::fmt::Debug for Fat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<FAT type={:?} length={}>", self.r#type, self.data.len())
    }
}

#[derive(Debug, Clone, Copy)]
enum FatEntry {
    /// Indicates a cluster is free
    Free,
    /// Indicates a cluster is the last one for a file.
    Eof,
    /// Indicates a defective cluster
    Defective,
    /// Indicates a cluster is not the last one for a file, and includes an index to the next cluster.
    HasNext(u32),
}

impl FatEntry {
    pub fn is_allocated(self) -> bool {
        matches!(self, FatEntry::Eof | FatEntry::HasNext(_))
    }
    pub fn is_free(self) -> bool {
        matches!(self, FatEntry::Free)
    }
    fn fat16_value(self) -> u16 {
        match self {
            Self::Free => 0,
            Self::Eof => 0xffff,
            Self::Defective => 0xfff7,
            Self::HasNext(n) => {
                assert!(
                    n < 0xfff7,
                    "fat16_value called on an invalid FAT-16 FAT entry"
                );
                n as u16
            }
        }
    }
    fn fat32_value(self) -> u32 {
        match self {
            Self::Free => 0,
            Self::Eof => 0xfff_ffff,
            Self::Defective => 0xfff_fff7,
            Self::HasNext(n) => {
                assert!(n < 0xfff_fff7, "fat32_value called on an invalid FAT entry");
                n
            }
        }
    }
}

impl Fat {
    pub fn new(
        device: &mut Block,
        cluster_count: u32,
        r#type: FatType,
        sectors: core::ops::Range<u32>,
        mirror_count: u32,
    ) -> Result<Self> {
        let sectors_per_fat = sectors.end - sectors.start;
        // read the FAT from disk.
        let mut data = vec![0u32; (sectors_per_fat * (BLOCK_SECTOR_SIZE as u32 / 4)) as usize];
        for (i, sector) in sectors.clone().enumerate() {
            device.read(
                sector,
                data[i * (BLOCK_SECTOR_SIZE / 4)..(i + 1) * (BLOCK_SECTOR_SIZE / 4)].as_bytes_mut(),
            )?;
        }

        let fat_entry_count = data.len() as u32 * if r#type == FatType::Fat16 { 2 } else { 1 };
        if fat_entry_count < cluster_count {
            return error!("FAT size is too small");
        }
        let free_clusters = FreeSet::new_all_allocated(cluster_count);
        let mut fat = Self {
            data,
            r#type,
            free_clusters,
            mirror_count,
            first_fat_sector: sectors.start,
            sectors_per_fat,
        };
        // the first two FAT entries are reserved
        for i in 2..cluster_count {
            let entry = fat.entry(i);
            if entry.is_free() {
                fat.free_clusters.free(i);
            }
            if let FatEntry::HasNext(n) = entry {
                if n < 2 || n >= cluster_count {
                    return error!(
                        "invalid entry in FAT: 0x{n:08x} (cluster count = {cluster_count})"
                    );
                }
            }
        }
        Ok(fat)
    }

    /// Allocate the first cluster in a file/directory.
    pub fn allocate_cluster(&mut self) -> Result<u32> {
        let cluster = self.free_clusters.allocate().ok_or(Error::NoSpace)?;
        self.set_entry(cluster, FatEntry::Eof);
        Ok(cluster)
    }

    /// Allocate a new cluster, and set `cluster` to point to it as the next cluster in the file.
    pub fn link_new_cluster(&mut self, cluster: u32) -> Result<u32> {
        let new_cluster = self.allocate_cluster()?;
        self.set_entry(cluster, FatEntry::HasNext(new_cluster));
        Ok(new_cluster)
    }

    /// Free this cluster
    pub fn free_cluster(&mut self, cluster: u32) {
        self.free_clusters.free(cluster);
        self.set_entry(cluster, FatEntry::Free);
    }

    /// Set this cluster as the last one in the file.
    ///
    /// IMPORTANT: Make sure you call [`Self::free_cluster`] on the clusters after this one — otherwise
    /// they will never be freed!
    pub fn set_eof(&mut self, cluster: u32) {
        self.set_entry(cluster, FatEntry::Eof);
    }

    fn set_entry(&mut self, i: u32, entry: FatEntry) {
        let i = i as usize;
        match self.r#type {
            FatType::Fat16 => {
                let raw_entry: u32 = entry.fat16_value().into();
                let e = &mut self.data[i / 2];
                *e = e.to_le(); // flip bytes if big-endian
                if i % 2 == 0 {
                    *e &= 0xffff_0000;
                    *e |= raw_entry;
                } else {
                    *e &= 0x0000_ffff;
                    *e |= raw_entry << 16;
                }
                *e = e.to_le(); // flip back bytes if big-endian
            }
            FatType::Fat32 => {
                let raw_entry = entry.fat32_value();
                // the top 4 bits of the FAT entry are reserved for
                // future extensions and should not be changed
                let e = &mut self.data[i];
                *e = e.to_le(); // flip bytes if big-endian
                *e &= 0xf000_0000;
                *e |= raw_entry;
                *e = e.to_le(); // flip back bytes if big-endian
            }
        }
    }

    fn entry(&self, i: u32) -> FatEntry {
        let i = i as usize;
        match self.r#type {
            FatType::Fat16 => {
                let raw_entry = if i % 2 == 0 {
                    self.data[i / 2].to_le() as u16
                } else {
                    (self.data[i / 2].to_le() >> 16) as u16
                };
                match raw_entry {
                    0 => FatEntry::Free,
                    0xFFF7 => FatEntry::Defective,
                    0xFFF8..=0xFFFF => FatEntry::Eof,
                    x => FatEntry::HasNext(x.into()),
                }
            }
            FatType::Fat32 => match self.data[i].to_le() & 0xFFF_FFFF {
                0 => FatEntry::Free,
                0xFFF_FFF7 => FatEntry::Defective,
                // in theory, this should just need to test for 0xfffffff, but
                // mkfs.vfat sets FAT[2] to 0xffffff8 for some reason.
                // according to the spec, 0xffffff8-0xffffffe:
                // "should not be used. May be interpreted as an allocated cluster and the final cluster in the file"
                0xFFF_FFF8..=0xFFF_FFFF => FatEntry::Eof,
                x => FatEntry::HasNext(x),
            },
        }
    }

    /// Get cluster after `cluster` in file.
    ///
    /// Returns `Ok(None)` if `cluster` is the last one in the file.
    pub fn next_cluster(&self, cluster: u32) -> Result<Option<u32>> {
        match self.entry(cluster) {
            FatEntry::Eof => Ok(None),
            FatEntry::HasNext(n) => Ok(Some(n)),
            FatEntry::Defective => {
                error!("defective cluster referenced in file")
            }
            FatEntry::Free => {
                error!("free cluster referenced in file")
            }
        }
    }
    pub fn is_cluster_allocated(&self, cluster: u32) -> bool {
        self.entry(cluster).is_allocated()
    }
    /// sync FAT changes to disk
    pub fn sync(&self, device: &mut Block) -> Result<()> {
        let data = self.data.as_bytes();
        for fat_index in 0..self.mirror_count {
            for sector in 0..self.sectors_per_fat {
                let sector = self.first_fat_sector + fat_index * self.sectors_per_fat + sector;
                device.write(
                    sector,
                    &data[sector as usize * BLOCK_SECTOR_SIZE
                        ..(sector + 1) as usize * BLOCK_SECTOR_SIZE],
                )?;
            }
        }
        Ok(())
    }
}
