mod dirent;
#[allow(clippy::module_inception)]
mod fat;
use crate::block::block_core::{Block, BLOCK_SECTOR_SIZE};
use crate::vfs::{
    DirEntries, Error, FileInfo, INodeNum, INodeType, Path, RawDirEntry, Result, SimpleFileSystem,
};
use alloc::{collections::BTreeMap, string::String, vec, vec::Vec};
use core::cmp::min;
use core::ops::Range;
use fat::Fat;
// These are little-endian unaligned integer types
use zerocopy::little_endian::{U16, U32};
use zerocopy::{FromBytes, FromZeroes, Unaligned};

#[derive(Debug)]
struct FatFileInfo {
    vfs: FileInfo,
    clusters: Vec<u32>,
}

// convenience macro for returning errors
macro_rules! error {
    ($($args:expr),*) => {
        Err(crate::vfs::Error::IO(alloc::format!($($args),*)))
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
    /// In-memory file information
    file_info: BTreeMap<INodeNum, FatFileInfo>,
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
    fn fat_mirroring_enabled(&self) -> bool {
        (u16::from(self.ext_flags) & (1 << 7)) != 0
    }
    fn active_fat_index(&self) -> u32 {
        if self.fat_mirroring_enabled() {
            u32::from(self.ext_flags) & 0xf
        } else {
            0
        }
    }
    fn verify_integrity(&self) -> Result<()> {
        let bk_boot_sector: u16 = self.bk_boot_sector.into();
        if bk_boot_sector != 0 && bk_boot_sector != 6 {
            return error!("Invalid value of BkBootSec: {bk_boot_sector}");
        }
        Ok(())
    }
}

impl FatFS {
    /// Create new FAT filesystem from block device
    pub fn new(mut block: Block) -> Result<Self> {
        let mut first_sector = [0; 512];
        block.read(0, &mut first_sector)?;
        let fat16_header: &Fat16Header =
            Fat16Header::ref_from(&first_sector).expect("Fat16Header type should be 512 bytes");
        // NOTE: signature is in sample place in FAT-16 and -32.
        if fat16_header.signature_word != [0x55, 0xAA] {
            return error!("missing FAT signature in first sector");
        }
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
        // size of a single FAT in bytes
        let mut fat_size: u32 = base_header.fat16_fat_size.into();
        if fat_size == 0 {
            fat_size = fat32_header.fat_size();
        }
        let num_fats = u32::from(base_header.num_fats);
        let total_sectors = base_header.total_sectors();
        // number of FAT sectors reserved for file/directory data
        let data_sectors =
            total_sectors - reserved_sector_count - num_fats * fat_size - root_dir_sectors;
        let cluster_count = data_sectors / u32::from(base_header.sectors_per_cluster);
        let fat_type;
        if cluster_count < 4085 {
            return error!("FAT-12 is not supported. Try creating a larger volume.");
        } else if cluster_count < 65525 {
            fat_type = FatType::Fat16;
        } else {
            fat32_header.verify_integrity()?;
            fat_type = FatType::Fat32;
        }
        let disk_sectors_per_fat_sector = bytes_per_sector / BLOCK_SECTOR_SIZE as u32;
        // First disk sector in the FAT.
        let mut fat_first_disk_sector = reserved_sector_count * disk_sectors_per_fat_sector;
        if fat_type == FatType::Fat32 && !fat32_header.fat_mirroring_enabled() {
            // In this case, there are multiple FATs, but only one of them is “active”,
            // for some reason.
            fat_first_disk_sector += fat32_header.active_fat_index() * fat_size;
        }
        // number of disk sectors taken up by a single FAT
        let fat_disk_sector_count = fat_size * disk_sectors_per_fat_sector;
        let fat = Fat::new(
            &mut block,
            cluster_count,
            fat_type,
            fat_first_disk_sector..fat_first_disk_sector + fat_disk_sector_count,
        )?;
        let fat16_first_root_disk_sector =
            reserved_sector_count * disk_sectors_per_fat_sector + fat_disk_sector_count * num_fats;
        let first_cluster_disk_sector =
            fat16_first_root_disk_sector + root_dir_sectors * disk_sectors_per_fat_sector;
        let root_inode: u32 = if fat_type == FatType::Fat32 {
            // for FAT-32, root is just like any other directory
            fat32_header.root_cluster.into()
        } else {
            // use an inode of 0 for the root directory (needs special handling)
            0
        };
        let disk_sectors_per_cluster =
            disk_sectors_per_fat_sector * u32::from(base_header.sectors_per_cluster);
        let root_info = FatFileInfo {
            vfs: FileInfo {
                inode: root_inode,
                size: 0,
                r#type: INodeType::Directory,
                nlink: 1,
            },
            clusters: fat.clusters_for_file(root_inode)?,
        };
        let mut file_info = BTreeMap::new();
        file_info.insert(root_inode, root_info);
        Ok(Self {
            block,
            fat,
            root_inode,
            file_info,
            disk_sectors_per_cluster,
            first_cluster_disk_sector,
            fat16_first_root_disk_sector,
            fat16_root_disk_sector_count: base_header.fat16_root_ent_count() * 32
                / BLOCK_SECTOR_SIZE as u32,
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
    /// Range of disk sectors reserved for the root directory.
    ///
    /// It's a bit strange that the root directory has its own space
    /// dedicated for it; this was removed in FAT-32 (and the
    /// root directory becomes just like any other directory).
    pub(super) fn fat16_root_disk_sectors(&self) -> Range<u32> {
        let first = self.fat16_first_root_disk_sector;
        first..first + self.fat16_root_disk_sector_count
    }
    fn cluster_size(&self) -> u32 {
        self.disk_sectors_per_cluster * BLOCK_SECTOR_SIZE as u32
    }
}

impl SimpleFileSystem for FatFS {
    fn root(&self) -> INodeNum {
        self.root_inode
    }
    fn open(&mut self, inode: INodeNum) -> Result<()> {
        if !self.fat.is_cluster_allocated(inode) {
            return Err(Error::NotFound);
        }
        debug_assert!(self.file_info.contains_key(&inode), "inode opened without its directory entry being read (or there is a bug in the FAT filesystem)");
        Ok(())
    }
    fn create(&mut self, _parent: INodeNum, _name: &Path) -> Result<INodeNum> {
        Err(Error::ReadOnlyFS)
    }
    fn mkdir(&mut self, _parent: INodeNum, _name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }
    fn unlink(&mut self, _parent: INodeNum, _name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }
    fn rmdir(&mut self, _parent: INodeNum, _name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }
    fn readdir(&mut self, dir: INodeNum) -> Result<DirEntries> {
        let (fat_entries, names) = dirent::read_directory(self, dir)?;
        let mut entries = vec![];
        for entry in &fat_entries {
            let inode = entry.info.inode;
            self.file_info.insert(
                inode,
                FatFileInfo {
                    vfs: entry.info.clone(),
                    clusters: self.fat.clusters_for_file(inode)?,
                },
            );
            entries.push(RawDirEntry {
                inode,
                r#type: entry.info.r#type,
                name: entry.name,
            });
        }
        Ok(DirEntries {
            filenames: names,
            entries,
        })
    }
    fn release(&mut self, _inode: INodeNum) {}
    fn read(&mut self, file: INodeNum, offset: u64, mut buf: &mut [u8]) -> Result<usize> {
        let Ok(mut offset) = u32::try_from(offset) else {
            // FAT files can't exceed 4GB, so if offset > u32::MAX, it's definitely past EOF
            return Ok(0);
        };
        let info = &self.file_info[&file];
        let file_size = info.vfs.size as u32;
        let mut read_count = 0;
        while !buf.is_empty() && offset < file_size {
            // read a single cluster from the file
            let cluster_index = offset / self.cluster_size();
            let cluster_offset = offset % self.cluster_size();
            let sector_within_cluster = cluster_offset % self.disk_sectors_per_cluster;
            let sector_offset = cluster_offset % BLOCK_SECTOR_SIZE as u32;
            let cluster = info.clusters[cluster_index as usize];
            let cluster_start = self.first_disk_sector_in_cluster(cluster);
            for sector in
                cluster_start + sector_within_cluster..cluster_start + self.disk_sectors_per_cluster
            {
                let mut sector_data = [0; BLOCK_SECTOR_SIZE];
                self.block.read(sector, &mut sector_data)?;
                // Read # of bytes equal to the minimum of:
                //   - the buffer size
                //   - the amount of bytes left in the file
                //   - the entire sector (starting from sector_offset)
                let read_size = min(
                    buf.len() as u32,
                    min(file_size - offset, BLOCK_SECTOR_SIZE as u32 - sector_offset),
                );
                buf[..read_size as usize].copy_from_slice(
                    &sector_data[sector_offset as usize..(sector_offset + read_size) as usize],
                );
                buf = &mut buf[read_size as usize..];
                offset += read_size;
                read_count += read_size;
            }
        }
        Ok(read_count as usize)
    }
    fn write(&mut self, _file: INodeNum, _offset: u64, _buf: &[u8]) -> Result<usize> {
        Err(Error::ReadOnlyFS)
    }
    fn stat(&mut self, file: INodeNum) -> Result<FileInfo> {
        Ok(self
            .file_info
            .get(&file)
            .expect("FAT inconsistency error")
            .vfs
            .clone())
    }
    fn link(&mut self, _source: INodeNum, _parent: INodeNum, _name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }
    fn symlink(&mut self, _link: &Path, _parent: INodeNum, _name: &Path) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }
    fn readlink(&mut self, _link: INodeNum) -> Result<String> {
        panic!("this should never be called by the kernel, since we never tell it something is a symlink")
    }
    fn truncate(&mut self, _file: INodeNum, _size: u64) -> Result<()> {
        Err(Error::ReadOnlyFS)
    }
    fn sync(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block::block_core::test::block_from_file;
    use crate::vfs::OwnedDirEntry;
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
    fn test_simple(mut fat: FatFS) {
        let root = fat.root();
        fat.open(root).unwrap();
        let entries: Vec<OwnedDirEntry> = fat.readdir(root).unwrap().to_sorted_vec();
        fn check_entry(entry: &OwnedDirEntry, name: &str, r#type: INodeType) {
            assert_eq!(&entry.name, name);
            assert_eq!(entry.r#type, r#type);
        }
        assert_eq!(entries.len(), 4);
        check_entry(&entries[0], "a", INodeType::File);
        check_entry(&entries[1], "b", INodeType::File);
        check_entry(&entries[2], "c", INodeType::File);
        check_entry(&entries[3], "d", INodeType::Directory);
        let dir_d = entries[3].inode;
        fat.open(dir_d).unwrap();
        let file_a = entries[0].inode;
        fat.open(file_a).unwrap();
        let mut buf = [0; 512];
        let n = fat.read(file_a, 0, &mut buf[..]).unwrap();
        assert_eq!(&buf[..n], b"file a\n");
        fat.release(file_a);
        let dir_d_entries = fat.readdir(dir_d).unwrap().to_sorted_vec();
        assert_eq!(dir_d_entries.len(), 1);
        check_entry(&dir_d_entries[0], "f", INodeType::File);
        let file_f = dir_d_entries[0].inode;
        fat.open(file_f).unwrap();
        let n = fat.read(file_f, 0, &mut buf[..]).unwrap();
        assert_eq!(&buf[..n], b"inner file\n");
        fat.release(file_f);
        fat.release(dir_d);
        fat.release(root);
    }
    #[test]
    fn simple_fat16() {
        // simple disk image, with no multi-cluster files or directories
        let fat = open_img_gz("tests/fat/simple_fat16.img.gz");
        test_simple(fat);
    }
    #[test]
    fn simple_fat32() {
        // FAT-32 version of simple_fat16.img.gz
        let fat = open_img_gz("tests/fat/simple_fat32.img.gz");
        test_simple(fat);
    }
    fn read_only_test_vs_host(name: &str, r#type: FatType) {
        let type_string = match r#type {
            FatType::Fat16 => "fat16",
            FatType::Fat32 => "fat32",
        };
        let mut fat = open_img_gz(&format!("tests/fat/{name}_{type_string}.img.gz"));
        crate::vfs::read_only_test::read_only_test(&mut fat, format!("tests/fat/{name}"));
    }
    #[test]
    fn long_names_fat16() {
        read_only_test_vs_host("long_names", FatType::Fat16);
    }
    #[test]
    fn long_names_fat32() {
        read_only_test_vs_host("long_names", FatType::Fat32);
    }

    fn large_file(r#type: FatType) {
        let type_string = match r#type {
            FatType::Fat16 => "fat16",
            FatType::Fat32 => "fat32",
        };
        let mut fat = open_img_gz(&format!("tests/fat/large_file_{type_string}.img.gz"));

        let root = fat.root();
        fat.open(root).unwrap();

        // Open the large file (assume it spans multiple clusters)
        let entries: Vec<OwnedDirEntry> = fat.readdir(root).unwrap().to_sorted_vec();
        let file_large = entries.iter().find(|e| e.name == "large_file.txt").unwrap();

        fat.open(file_large.inode).unwrap();

        // Buffer for reading file content
        let mut buf = vec![0; 128 * 1024];  // 128KB buffer
        let n = fat.read(file_large.inode, 0, &mut buf).unwrap();

         // Ensure that the file read more than 64KB
        assert!(n > 64 * 1024, "Expected to read more than 64KB, but read {} bytes", n);

        fat.release(file_large.inode);
        fat.release(root);
    }
    #[test]
    fn large_file_fat16() {
        large_file(FatType::Fat16);
    }
    #[test]
    fn large_file_fat32() {
        large_file(FatType::Fat32);
    }

    fn large_dir(r#type: FatType) {
        let type_string = match r#type {
            FatType::Fat16 => "fat16",
            FatType::Fat32 => "fat32",
        };
        let mut fat = open_img_gz(&format!("tests/fat/large_dir_{type_string}.img.gz"));

        let root = fat.root();
        fat.open(root).unwrap();

        // Open the large directory (assume it spans multiple clusters)
        let entries: Vec<OwnedDirEntry> = fat.readdir(root).unwrap().to_sorted_vec();

        // Check if we have a large number of entries
        assert!(entries.len() > 299, "Expected more than 299 entries, found {}", entries.len());
        
        // Read the directory entries
        fat.release(root);
    }

    #[test]
    fn large_dir_fat16() {
        large_dir(FatType::Fat16);
    }

    #[test]
    fn large_dir_fat32() {
        large_dir(FatType::Fat32);
    }
    
}
