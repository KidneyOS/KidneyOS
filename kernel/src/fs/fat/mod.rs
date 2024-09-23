mod dirent;
#[allow(clippy::module_inception)]
mod fat;
use crate::block::block_core::{Block, BLOCK_SECTOR_SIZE};
use crate::vfs::{Error, FileHandle, FileInfo, FileSystem, INodeNum, INodeType, Path, Result};
use alloc::collections::BTreeMap;
use core::cmp::min;
use core::ops::Range;
use dirent::Directory;
use fat::{Fat, FatEntry};
// These are little-endian unaligned integer types
use zerocopy::little_endian::{U16, U32};
use zerocopy::{FromBytes, FromZeroes, Unaligned};

#[derive(Debug, Clone, Copy)]
pub struct FatFileHandle {
    inode: INodeNum,
    file_offset: u32,
    curr_cluster: u32,
    version_number: u64,
}

#[derive(Debug)]
struct FatFileInfo {
    vfs: FileInfo,
    version_number: u64,
}

impl FileHandle for FatFileHandle {
    fn inode(self) -> INodeNum {
        self.inode
    }
}

impl FatFileHandle {
    fn advance_cluster(&mut self, fs: &FatFS) -> Result<bool> {
        if self.curr_cluster == u32::MAX {
            return Ok(false);
        }
        match fs.fat.entry(self.curr_cluster) {
            FatEntry::Eof => {
                self.curr_cluster = u32::MAX;
                Ok(false)
            }
            FatEntry::Defective => {
                self.curr_cluster = u32::MAX;
                error!("defective cluster referenced in file")
            }
            FatEntry::Free => {
                self.curr_cluster = u32::MAX;
                error!("free cluster referenced in file")
            }
            FatEntry::HasNext(n) => {
                self.curr_cluster = n;
                Ok(true)
            }
        }
    }
}

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
    cached_directories: BTreeMap<INodeNum, Directory>,
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
        let root_info = FatFileInfo {
            vfs: FileInfo {
                inode: root_inode,
                size: 0,
                r#type: INodeType::Directory,
                nlink: 1,
            },
            version_number: 0,
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
        fn insert_directory_if_missing(
            fs: &mut FatFS,
            inode: INodeNum,
            cached_directories: &mut BTreeMap<INodeNum, Directory>,
        ) -> Result<()> {
            use alloc::collections::btree_map::Entry;
            if let Entry::Vacant(v) = cached_directories.entry(inode) {
                let directory = Directory::read(fs, inode)?;
                for (_, info) in directory.entries() {
                    let inode = info.inode;
                    fs.file_info.insert(
                        inode,
                        FatFileInfo {
                            version_number: 0,
                            vfs: info.clone(),
                        },
                    );
                }
                v.insert(directory);
            }
            Ok(())
        }
        // temporarily move out cached_directories to appease the borrow checker
        let mut cached_directories = core::mem::take(&mut self.cached_directories);
        let result = insert_directory_if_missing(self, inode, &mut cached_directories);
        self.cached_directories = cached_directories;
        result?;
        Ok(&self.cached_directories[&inode])
    }
}

impl FileSystem for FatFS {
    type FileHandle = FatFileHandle;
    type DirectoryIterator<'a> = dirent::FatDirectoryIterator<'a>;
    fn root(&self) -> INodeNum {
        self.root_inode
    }
    fn lookup(&mut self, parent: &mut FatFileHandle, name: &Path) -> Result<INodeNum> {
        let dir = self.get_directory(parent.inode)?;
        dir.lookup(name).ok_or(Error::NotFound)
    }
    fn open(&mut self, inode: INodeNum) -> Result<FatFileHandle> {
        let fat_entry = self.fat.entry(inode);
        if !fat_entry.is_allocated() {
            return Err(Error::NotFound);
        }
        let info = self
            .file_info
            .get(&inode)
            .expect("FAT consistency error: inode not in file_info");
        Ok(FatFileHandle {
            inode,
            curr_cluster: inode,
            file_offset: 0,
            version_number: info.version_number,
        })
    }
    fn create(&mut self, _parent: &mut FatFileHandle, _name: &Path) -> Result<FatFileHandle> {
        todo!()
    }
    fn mkdir(&mut self, _parent: &mut FatFileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn unlink(&mut self, _parent: &mut FatFileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn rmdir(&mut self, _parent: &mut FatFileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn readdir(&mut self, dir: &mut FatFileHandle, offset: u64) -> Self::DirectoryIterator<'_> {
        let dir = self.get_directory(dir.inode);
        dirent::FatDirectoryIterator::new(dir, offset)
    }
    fn release(&mut self, inode: INodeNum) {
        if inode == self.root() {
            // don't ever remove root from cache.
            return;
        }
        if let Some(mut dir) = self.cached_directories.remove(&inode) {
            // just ignore any disk errors, at least for now.
            let _ = dir.sync(self);
        }
    }
    fn read(&mut self, file: &mut FatFileHandle, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let Ok(offset) = u32::try_from(offset) else {
            // FAT files can't exceed 4GB, so if offset > u32::MAX, it's definitely past EOF
            return Ok(0);
        };
        let cluster_size = self.disk_sectors_per_cluster * BLOCK_SECTOR_SIZE as u32;
        let curr_cluster_index = file.file_offset / cluster_size;
        let new_cluster_index = offset / cluster_size;
        let info = self
            .file_info
            .get(&file.inode())
            .expect("FAT consistency error: file not in file_info");
        let file_size: u32 = info.vfs.size.try_into().expect("FAT files should be <4GB");
        if file.version_number != info.version_number || new_cluster_index < curr_cluster_index {
            // need to recompute cluster from the start for backwards access,
            // or if the file has been concurrently modified
            file.curr_cluster = file.inode();
            for _ in 0..new_cluster_index {
                if !file.advance_cluster(self)? {
                    // end-of-file reached
                    return Ok(0);
                }
            }
        } else if new_cluster_index > curr_cluster_index {
            // advance to new cluster
            for _ in 0..new_cluster_index - curr_cluster_index {
                if !file.advance_cluster(self)? {
                    // end-of-file reached
                    return Ok(0);
                }
            }
        } else if file.curr_cluster == u32::MAX {
            // already reached end-of-file
            return Ok(0);
        }
        // file.curr_cluster should now be correct
        let mut offset = offset;
        let mut buf = buf;
        let mut read_count = 0;
        let mut sector_data = [0; BLOCK_SECTOR_SIZE];
        while !buf.is_empty() {
            let cluster_offset = offset % cluster_size;
            let sector_offset = offset % BLOCK_SECTOR_SIZE as u32;
            let disk_sector = self.first_disk_sector_in_cluster(file.curr_cluster)
                + cluster_offset / BLOCK_SECTOR_SIZE as u32;
            self.block.read(disk_sector, &mut sector_data)?;
            let mut n = min(buf.len() as u32, BLOCK_SECTOR_SIZE as u32 - sector_offset);
            n = min(n, file_size - offset);
            buf[..n as usize].copy_from_slice(
                &sector_data[sector_offset as usize..(sector_offset + n) as usize],
            );
            offset += n;
            read_count += n;
            buf = &mut buf[n as usize..];
            if offset >= file_size {
                break;
            }
            if offset % cluster_size == 0 {
                // new cluster
                if !file.advance_cluster(self)? {
                    return error!(
                        "corrupt FAT filesystem: file size exceeds number of allocated clusters"
                    );
                }
            }
        }
        Ok(read_count as usize)
    }
    fn write(&mut self, _file: &mut FatFileHandle, _offset: u64, _buf: &[u8]) -> Result<usize> {
        todo!()
    }
    fn stat(&mut self, file: &FatFileHandle) -> Result<FileInfo> {
        Ok(self
            .file_info
            .get(&file.inode)
            .expect("FAT inconsistency error")
            .vfs
            .clone())
    }
    fn link(
        &mut self,
        _source: &mut FatFileHandle,
        _parent: &mut FatFileHandle,
        _name: &Path,
    ) -> Result<()> {
        Err(Error::Unsupported)
    }
    fn symlink(&mut self, _link: &Path, _parent: &mut FatFileHandle, _name: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    fn readlink<'a>(
        &mut self,
        _link: &mut FatFileHandle,
        _buf: &'a mut Path,
    ) -> Result<Option<&'a str>> {
        panic!("this should never be called by the kernel, since we never tell it something is a symlink")
    }
    fn truncate(&mut self, _file: &mut FatFileHandle, _size: u64) -> Result<()> {
        todo!()
    }
    fn sync(&mut self) -> Result<()> {
        fn sync_directories(
            fs: &mut FatFS,
            directories: &mut BTreeMap<INodeNum, Directory>,
        ) -> Result<()> {
            for dir in directories.values_mut() {
                dir.sync(fs)?;
            }
            Ok(())
        }
        // temporarily move out cached_directories to appease the borrow checker
        let mut directories = core::mem::take(&mut self.cached_directories);
        let result = sync_directories(self, &mut directories);
        self.cached_directories = directories;
        result?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block::block_core::test::block_from_file;
    use crate::vfs::{DirectoryIterator, OwnedDirEntry};
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
        let mut root = fat.open(fat.root()).unwrap();
        let mut it = fat.readdir(&mut root, 0);
        let mut entries = vec![];
        while let Some(entry) = it.next().unwrap() {
            entries.push(entry.to_owned());
        }
        entries.sort_by_key(|e| e.name.clone());
        fn check_entry(entry: &OwnedDirEntry, name: &str, r#type: INodeType) {
            assert_eq!(&entry.name, name);
            assert_eq!(entry.r#type, r#type);
        }
        check_entry(&entries[0], "a", INodeType::File);
        check_entry(&entries[1], "b", INodeType::File);
        check_entry(&entries[2], "c", INodeType::File);
        check_entry(&entries[3], "d", INodeType::Directory);
        let mut dir_d = fat.open(entries[3].inode).unwrap();
        let file_a_inode = fat.lookup(&mut root, "a").unwrap();
        let mut file_a = fat.open(file_a_inode).unwrap();
        let mut buf = [0; 512];
        let n = fat.read(&mut file_a, 0, &mut buf[..]).unwrap();
        assert_eq!(&buf[..n], b"file a\n");
        fat.release(file_a.inode);
        let file_f_inode = fat.lookup(&mut dir_d, "f").unwrap();
        let mut file_f = fat.open(file_f_inode).unwrap();
        let n = fat.read(&mut file_f, 0, &mut buf[..]).unwrap();
        assert_eq!(&buf[..n], b"inner file\n");
        fat.release(file_f.inode);
        fat.release(dir_d.inode);
        fat.release(root.inode);
    }
    #[test]
    fn simple_fat16() {
        let fat = open_img_gz("tests/fat/simple_fat16.img.gz");
        test_simple(fat);
    }
    #[test]
    fn simple_fat32() {
        let fat = open_img_gz("tests/fat/simple_fat32.img.gz");
        test_simple(fat);
    }
}
