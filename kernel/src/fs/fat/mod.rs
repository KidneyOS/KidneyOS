mod dirent;
#[allow(clippy::module_inception)]
mod fat;
use crate::block::block_core::{Block, BLOCK_SECTOR_SIZE};
use crate::vfs::{Error, FileHandle, FileInfo, FileSystem, INodeNum, Path, Result};
use alloc::collections::BTreeMap;
use core::ops::Range;
use dirent::Directory;
use fat::Fat;
// These are little-endian unaligned integer types
use zerocopy::little_endian::{U16, U32};
use zerocopy::{FromBytes, FromZeroes, Unaligned};

// convenience macro for returning errors
macro_rules! error {
    ($($args:expr),*) => {
        Err(crate::vfs::Error::IO(format!($($args),*)))
    }
}
pub(super) use error;

/// A FAT-16 or FAT-32 filesystem
pub struct FatFS {
    /// Underlying block device
    block: Block,
    /// Cluster number of root
    root_inode: INodeNum,
    /// First sector number of root directory entries (FAT-12/16 only)
    fat16_first_root_disk_sector: u32,
    /// Number of disk sectors reserved for root directory (FAT-12/16 only)
    fat16_root_disk_sector_count: u32,
    /// Number of disk sectors (size = `BLOCK_SECTOR_SIZE`) per FAT cluster
    disk_sectors_per_cluster: u32,
    /// Disk sector which contains the start of the first FAT cluster
    first_cluster_disk_sector: u32,
    /// File allocation table
    fat: Fat,
    /// In-memory copies of directory entries
    cached_directories: BTreeMap<INodeNum, Result<Directory>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FatType {
    Fat16,
    Fat32,
}

// Base BPB (BIOS Parameter Block) for a FAT 16/32 filesystem
#[repr(C)]
#[allow(dead_code)]
#[derive(FromZeroes, FromBytes, Unaligned)]
struct FatBaseHeader {
    jmp_boot: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_sector: U16,
    sectors_per_cluster: u8,
    reserved_sector_count: U16,
    num_fats: u8,
    fat16_root_ent_count: U16,
    total_sectors16: U16,
    media: u8,
    fat16_fat_size: U16,
    sectors_per_track: U16,
    num_heads: U16,
    hidden_sectors: U32,
    total_sectors32: U32,
}

impl FatBaseHeader {
    fn bytes_per_sector(&self) -> u32 {
        self.bytes_per_sector.into()
    }
    fn reserved_sector_count(&self) -> u32 {
        self.reserved_sector_count.into()
    }
    fn total_sectors(&self) -> u32 {
        let total_sectors16: u16 = self.total_sectors16.into();
        if total_sectors16 == 0 {
            self.total_sectors32.into()
        } else {
            total_sectors16.into()
        }
    }
    fn fat16_root_ent_count(&self) -> u32 {
        self.fat16_root_ent_count.into()
    }
    fn check_integrity(&self) -> Result<()> {
        if !matches!(self.bytes_per_sector(), 512 | 1024 | 2048 | 4096) {
            return error!(
                "invalid number of bytes per sector: {}",
                self.bytes_per_sector
            );
        }
        if !self.sectors_per_cluster.is_power_of_two() {
            return error!(
                "number of sectors per cluster ({}) is not a power of two",
                self.sectors_per_cluster
            );
        }
        if self.num_fats == 0 {
            return error!("must have at least one FAT");
        }
        if self.reserved_sector_count() == 0 {
            return error!("reserved sector count must be nonzero");
        }
        if self.fat16_root_ent_count() * 32 % self.bytes_per_sector() != 0 {
            return error!("root entry count * 32B must be an integer number of sectors");
        }
        Ok(())
    }
}

#[repr(C)]
#[allow(dead_code)]
#[derive(FromZeroes, FromBytes, Unaligned)]
struct Fat16Header {
    base: FatBaseHeader,
    drive_num: u8,
    _reserved: u8,
    boot_signature: u8,
    volume_id: U32,
    volume_label: [u8; 11],
    fs_type: [u8; 8],
    _unused: [u8; 448],
    signature_word: [u8; 2],
}

#[repr(C)]
#[allow(dead_code)]
#[derive(FromZeroes, FromBytes, Unaligned)]
struct Fat32Header {
    base: FatBaseHeader,
    fat_size: U32,
    ext_flags: U16,
    fs_version: U16,
    root_cluster: U32,
    fs_info: U16,
    bk_boot_sector: U16,
    _reserved: [u8; 12],
    drive_num: u8,
    _reserved1: u8,
    boot_signature: u8,
    volume_id: U32,
    volume_label: [u8; 11],
    fs_type: [u8; 8],
    _unused: [u8; 420],
    signature_word: [u8; 2],
}

impl Fat32Header {
    fn fat_size(&self) -> u32 {
        self.fat_size.into()
    }
}

impl FatFS {
    /// Create new FAT filesystem from block device
    pub fn new(mut block: Block) -> Result<Self> {
        // TODO: check FAT signatures
        let mut first_sector = [0; 512];
        block.read(0, &mut first_sector)?;
        let fat16_header: &Fat16Header =
            Fat16Header::ref_from(&first_sector).expect("Fat16Header type should be 512 bytes");
        let fat32_header: &Fat32Header =
            Fat32Header::ref_from(&first_sector).expect("Fat32Header type should be 512 bytes");
        let base_header: &FatBaseHeader = &fat16_header.base;
        base_header.check_integrity()?;
        let reserved_sector_count: u32 = base_header.reserved_sector_count();
        let bytes_per_sector: u32 = base_header.bytes_per_sector();
        // very strangely, although there are many easy-to-detect differences
        // between FAT 16 and 32, the "correct" way to determine the type is
        // quite elaborate.

        // this will always be zero for FAT32
        let root_dir_sectors =
            (u32::from(base_header.fat16_root_ent_count) * 32).div_ceil(bytes_per_sector);
        let mut fat_size: u32 = base_header.fat16_fat_size.into();
        if fat_size == 0 {
            fat_size = fat32_header.fat_size();
        }
        let num_fats = u32::from(base_header.num_fats);
        let total_sectors = base_header.total_sectors();
        let data_sectors =
            total_sectors - reserved_sector_count - num_fats * fat_size - root_dir_sectors;
        let cluster_count = data_sectors / u32::from(base_header.sectors_per_cluster);
        let fat_type;
        if cluster_count < 4085 {
            return error!("FAT-12 is not supported. Try creating a larger volume.");
        } else if cluster_count < 65525 {
            fat_type = FatType::Fat16;
        } else {
            fat_type = FatType::Fat32;
        }
        let disk_sectors_per_fat_sector = bytes_per_sector / BLOCK_SECTOR_SIZE as u32;
        let fat_first_disk_sector = reserved_sector_count * disk_sectors_per_fat_sector;
        let fat_disk_sector_count = fat_size * disk_sectors_per_fat_sector;
        let fat = Fat::new(
            &mut block,
            cluster_count,
            fat_type,
            fat_first_disk_sector..fat_first_disk_sector + fat_disk_sector_count,
        )?;
        let fat16_first_root_disk_sector = fat_first_disk_sector + fat_disk_sector_count * num_fats;
        let first_cluster_disk_sector =
            fat16_first_root_disk_sector + root_dir_sectors * disk_sectors_per_fat_sector;
        let root_inode: u32 = if fat_type == FatType::Fat32 {
            fat32_header.root_cluster.into()
        } else {
            0
        };
        let disk_sectors_per_cluster =
            disk_sectors_per_fat_sector * u32::from(base_header.sectors_per_cluster);
        Ok(Self {
            block,
            fat,
            root_inode,
            disk_sectors_per_cluster,
            first_cluster_disk_sector,
            fat16_first_root_disk_sector,
            fat16_root_disk_sector_count: base_header.fat16_root_ent_count() * 32
                / BLOCK_SECTOR_SIZE as u32,
            cached_directories: BTreeMap::new(),
        })
    }
    fn first_disk_sector_in_cluster(&self, cluster: u32) -> u32 {
        assert!(cluster >= 2);
        self.first_cluster_disk_sector + (cluster - 2) * self.disk_sectors_per_cluster
    }
    pub(super) fn disk_sectors_in_cluster(&self, cluster: u32) -> Range<u32> {
        let first = self.first_disk_sector_in_cluster(cluster);
        first..first + self.disk_sectors_per_cluster
    }
    pub(super) fn fat16_root_disk_sectors(&self) -> Range<u32> {
        let first = self.fat16_first_root_disk_sector;
        first..first + self.fat16_root_disk_sector_count
    }
    fn get_directory(&mut self, inode: INodeNum) -> Result<&Directory> {
        // clippy wants us to use or_insert_with, but we can't because we need to
        // mutably borrow self to read the directory.
        #[allow(clippy::map_entry)]
        if !self.cached_directories.contains_key(&inode) {
            let directory = Directory::read(self, inode);
            self.cached_directories.insert(inode, directory);
        }
        self.cached_directories[&inode]
            .as_ref()
            .map_err(|e| e.clone())
    }
}

impl FileSystem for FatFS {
    type DirectoryIterator<'a> = dirent::FatDirectoryIterator<'a>;
    fn root(&self) -> INodeNum {
        self.root_inode
    }
    fn lookup(&mut self, parent: FileHandle, name: &Path) -> Result<INodeNum> {
        let dir = self.get_directory(parent.inode)?;
        dir.lookup(name).ok_or(Error::NotFound)
    }
    fn open(&mut self, inode: INodeNum) -> Result<FileHandle> {
        let fat_entry = self.fat.entry(inode);
        if !fat_entry.is_allocated() {
            return Err(Error::NotFound);
        }
        Ok(FileHandle { inode })
    }
    fn create(&mut self, _parent: FileHandle, _name: &Path) -> Result<FileHandle> {
        todo!()
    }
    fn mkdir(&mut self, _parent: FileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn unlink(&mut self, _parent: FileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn rmdir(&mut self, _parent: FileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn readdir(&mut self, dir: FileHandle, offset: u64) -> Self::DirectoryIterator<'_> {
        let dir = self.get_directory(dir.inode);
        dirent::FatDirectoryIterator::new(dir, offset)
    }
    fn release(&mut self, _inode: INodeNum) {
        todo!()
    }
    fn read(&mut self, _file: FileHandle, _offset: u64, _buf: &mut [u8]) -> Result<usize> {
        todo!()
    }
    fn write(&mut self, _file: FileHandle, _offset: u64, _buf: &[u8]) -> Result<usize> {
        todo!()
    }
    fn stat(&mut self, _file: FileHandle) -> Result<FileInfo> {
        todo!()
    }
    fn link(&mut self, _source: FileHandle, _parent: FileHandle, _name: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    fn symlink(&mut self, _link: &Path, _parent: FileHandle, _name: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    fn readlink<'a>(&mut self, _link: FileHandle, _buf: &'a mut Path) -> Result<Option<&'a str>> {
        panic!("this should never be called by the kernel, since we never tell it something is a symlink")
    }
    fn truncate(&mut self, _file: FileHandle, _size: u64) -> Result<()> {
        todo!()
    }
    fn sync(&mut self) -> Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block::block_core::test::block_from_file;
    use crate::vfs::DirectoryIterator;
    use std::fs::File;
    use std::io::{prelude::*, Cursor};
    /// Open a gzip-compressed raw disk image containing a FAT filesystem.
    /// Any changes made to the filesystem are kept in memory, but not written back to the file.
    fn open_img_gz(path: &str) -> FatFS {
        let file = File::open(path).unwrap();
        let mut gz_decoder = flate2::read::GzDecoder::new(file);
        let mut buf = vec![];
        gz_decoder.read_to_end(&mut buf).unwrap();
        FatFS::new(block_from_file(Cursor::new(buf))).unwrap()
    }
    #[test]
    fn test() {
        let mut fat = open_img_gz("tests/fat16.img.gz");
        let root = fat.open(fat.root()).unwrap();
        let mut it = fat.readdir(root, 0);
        while let Some(entry) = it.next().unwrap() {
            println!("{entry:?}");
        }
        panic!();
    }
}
