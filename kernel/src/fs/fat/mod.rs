mod dirent;
#[allow(clippy::module_inception)]
pub mod fat;
mod free_set;
use crate::block::block_core::{Block, BLOCK_SECTOR_SIZE};
use crate::vfs::{
    DirEntries, Error, FileHandle, FileInfo, FileSystem, INodeNum, INodeType, Path, RawDirEntry,
    Result,
};
use alloc::{collections::BTreeMap, vec};
use core::cmp::{max, min};
use core::ops::Range;
use fat::Fat;
// These are little-endian unaligned integer types
use zerocopy::little_endian::{U16, U32};
use zerocopy::{FromBytes, FromZeroes, Unaligned};

#[derive(Debug, Clone, Copy)]
pub struct FatFileHandle {
    inode: INodeNum,
    file_offset: u32,
    curr_cluster: u32,
    at_eof: bool,
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
        if self.at_eof {
            return Ok(false);
        }
        let next_cluster = fs.fat.next_cluster(self.curr_cluster)?;
        match next_cluster {
            Some(c) => {
                self.curr_cluster = c;
                Ok(true)
            }
            None => {
                self.at_eof = true;
                Ok(false)
            }
        }
    }
    fn seek_to_eof(&mut self, fs: &FatFS) -> Result<()> {
        while self.advance_cluster(fs)? {
            // (do nothing)
        }
        Ok(())
    }
    fn seek_to_cluster(&mut self, fs: &mut FatFS, cluster_index: u32) -> Result<()> {
        let curr_cluster_index = self.file_offset.div_ceil(fs.cluster_size());
        if fs.file_info[&self.inode].version_number != self.version_number
            || cluster_index < curr_cluster_index
        {
            // need to recompute cluster from the start for backwards access,
            // or if the file has been concurrently modified
            self.curr_cluster = self.inode;
            self.at_eof = false;
            for _ in 0..cluster_index {
                if !self.advance_cluster(fs)? {
                    // end-of-file reached
                    break;
                }
            }
        } else if cluster_index > curr_cluster_index && !self.at_eof {
            // advance to new cluster
            for _ in 0..cluster_index - curr_cluster_index {
                if !self.advance_cluster(fs)? {
                    // end-of-file reached
                    break;
                }
            }
        }
        Ok(())
    }
}

// convenience macro for returning errors
macro_rules! error {
    ($($args:expr),*) => {
        Err(crate::vfs::Error::IO(alloc::format!($($args),*)))
    }
}
use error;

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
pub enum FatType {
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
        let fat_mirroring_enabled =
            fat_type != FatType::Fat32 || fat32_header.fat_mirroring_enabled();
        let fat = Fat::new(
            &mut block,
            cluster_count,
            fat_type,
            fat_first_disk_sector..fat_first_disk_sector + fat_disk_sector_count,
            if fat_mirroring_enabled { num_fats } else { 1 },
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
        })
    }
    fn first_disk_sector_in_cluster(&self, cluster: u32) -> u32 {
        assert!(cluster >= 2);
        self.first_cluster_disk_sector + (cluster - 2) * self.disk_sectors_per_cluster
    }
    pub(self) fn disk_sectors_in_cluster(&self, cluster: u32) -> Range<u32> {
        let first = self.first_disk_sector_in_cluster(cluster);
        first..first + self.disk_sectors_per_cluster
    }
    /// Range of disk sectors reserved for the root directory.
    ///
    /// It's a bit strange that the root directory has its own space
    /// dedicated for it; this was removed in FAT-32 (and the
    /// root directory becomes just like any other directory).
    pub(self) fn fat16_root_disk_sectors(&self) -> Range<u32> {
        let first = self.fat16_first_root_disk_sector;
        first..first + self.fat16_root_disk_sector_count
    }
    fn cluster_size(&self) -> u32 {
        self.disk_sectors_per_cluster * BLOCK_SECTOR_SIZE as u32
    }
    fn read_file_sector(
        &mut self,
        file: &mut FatFileHandle,
        index: u32,
        data: &mut [u8],
    ) -> Result<u32> {
        let file_size = self.file_info[&file.inode].vfs.size as u32;
        if index > u32::MAX / BLOCK_SECTOR_SIZE as u32 {
            // must be end-of-file since FAT-32 files are 4GB max
            return Ok(0);
        }
        let file_offset = index * BLOCK_SECTOR_SIZE as u32;
        let cluster_index = index / self.disk_sectors_per_cluster;
        let sector_index_within_cluster = index % self.disk_sectors_per_cluster;
        file.seek_to_cluster(self, cluster_index)?;
        if file.at_eof {
            // EOF reached
            return Ok(0);
        }
        let first_sector_in_cluster = self.first_disk_sector_in_cluster(file.curr_cluster);
        let sector = first_sector_in_cluster + sector_index_within_cluster;
        self.block.read(sector, data)?;
        let read_count = min(BLOCK_SECTOR_SIZE as u32, file_size - file_offset);
        data[read_count as usize..].fill(0); // make sure uninitialized data isn't leaked
        Ok(read_count)
    }
    fn write_file_sector(
        &mut self,
        file: &mut FatFileHandle,
        index: u32,
        data: &[u8],
    ) -> Result<()> {
        let cluster_index = index / self.disk_sectors_per_cluster;
        let sector_index_within_cluster = index % self.disk_sectors_per_cluster;
        file.seek_to_cluster(self, cluster_index)?;
        assert!(!file.at_eof, "append_disk_sector should be called instead of write_disk_sector for writing past end of file");
        let first_sector_in_cluster = self.first_disk_sector_in_cluster(file.curr_cluster);
        let sector = first_sector_in_cluster + sector_index_within_cluster;
        self.block.write(sector, data)?;
        Ok(())
    }
    fn append_file_cluster(&mut self, file: &mut FatFileHandle) -> Result<()> {
        file.seek_to_eof(self)?;
        self.fat.link_new_cluster(file.curr_cluster)?;
        file.at_eof = false;
        let _advanced = file.advance_cluster(self)?;
        debug_assert!(_advanced, "this should succeed since we added a new sector");
        Ok(())
    }
}

impl FileSystem for FatFS {
    type FileHandle = FatFileHandle;
    fn root(&self) -> INodeNum {
        self.root_inode
    }
    fn open(&mut self, inode: INodeNum) -> Result<FatFileHandle> {
        if !self.fat.is_cluster_allocated(inode) {
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
            at_eof: false,
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
    fn readdir(&mut self, dir: &mut FatFileHandle) -> Result<DirEntries> {
        let (fat_entries, names) = dirent::read_directory(self, dir.inode)?;
        let mut entries = vec![];
        for entry in &fat_entries {
            let inode = entry.info.inode;
            self.file_info.insert(
                inode,
                FatFileInfo {
                    version_number: 0,
                    vfs: entry.info.clone(),
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
    fn read(&mut self, file: &mut FatFileHandle, offset: u64, mut buf: &mut [u8]) -> Result<usize> {
        let Ok(mut offset) = u32::try_from(offset) else {
            // FAT files can't exceed 4GB, so if offset > u32::MAX, it's definitely past EOF
            return Ok(0);
        };
        if buf.is_empty() {
            return Ok(0);
        }
        if buf.len() as u64 > (u32::MAX - offset) as u64 {
            // ensure offset + buf.len() doesn't exceed u32::MAX
            buf = &mut buf[..(u32::MAX - offset) as usize];
        }

        let mut read_count = 0;
        if offset % BLOCK_SECTOR_SIZE as u32 != 0 {
            // align to disk sector
            let mut sector = [0; BLOCK_SECTOR_SIZE];
            let offset_in_sector = offset % BLOCK_SECTOR_SIZE as u32;
            let bytes_read_from_sector =
                self.read_file_sector(file, offset / BLOCK_SECTOR_SIZE as u32, &mut sector)?;
            let mut n = min(
                buf.len() as u32,
                BLOCK_SECTOR_SIZE as u32 - offset_in_sector,
            );
            n = min(n, bytes_read_from_sector.saturating_sub(offset_in_sector));
            read_count += n as u32;
            offset += n as u32;
            buf[..n as usize].copy_from_slice(
                &sector[offset_in_sector as usize..(offset_in_sector + n) as usize],
            );
            buf = &mut buf[n as usize..];
            if bytes_read_from_sector < BLOCK_SECTOR_SIZE as u32 {
                return Ok(read_count as usize);
            }
        }
        debug_assert_eq!(offset % BLOCK_SECTOR_SIZE as u32, 0);
        while buf.len() >= BLOCK_SECTOR_SIZE {
            // read whole disk sectors
            let n = self.read_file_sector(
                file,
                offset / BLOCK_SECTOR_SIZE as u32,
                &mut buf[..BLOCK_SECTOR_SIZE],
            )?;
            read_count += n;
            offset += n;
            buf = &mut buf[n as usize..];
            if n < BLOCK_SECTOR_SIZE as u32 {
                // EOF reached
                return Ok(read_count as usize);
            }
        }
        if buf.is_empty() {
            return Ok(read_count as usize);
        }
        debug_assert_eq!(offset % BLOCK_SECTOR_SIZE as u32, 0);
        // read final few bytes
        let mut sector = [0; BLOCK_SECTOR_SIZE];
        let n = self.read_file_sector(file, offset / BLOCK_SECTOR_SIZE as u32, &mut sector)?;
        buf.copy_from_slice(&sector[..buf.len()]);
        read_count += n;
        Ok(read_count as usize)
    }
    fn write(&mut self, file: &mut FatFileHandle, offset: u64, buf: &[u8]) -> Result<usize> {
        // an offset > 2^32 is effectively the same as offset = 2^32
        //  (we just extend the file to 2^32 - 1 bytes in either case)
        let offset = min(offset, 0x1_0000_0000);
        let prev_file_size = self.file_info[&file.inode].vfs.size as u32;
        let new_file_size = max(
            prev_file_size,
            min(offset + buf.len() as u64, 0xffff_ffff) as u32,
        );
        if new_file_size > prev_file_size {
            let prev_cluster_count = prev_file_size.div_ceil(self.cluster_size());
            let new_cluster_count = new_file_size.div_ceil(self.cluster_size());
            // copy file handle, so that file's offset/cluster remains intact
            let mut handle = *file;
            handle.seek_to_eof(self)?;
            for _ in 0..new_cluster_count - prev_cluster_count {
                // append a cluster to the file
                self.append_file_cluster(&mut handle)?;
            }
            self.file_info.get_mut(&file.inode).unwrap().vfs.size = new_file_size.into();
            if offset > u64::from(prev_file_size) {
                // zero data from prev_file_size..offset
                // this isn't the most efficient way of doing things,
                // but writing beyond the end-of-file is rare anyways.
                let buf = [0; 4096];
                let mut zero_offset = u64::from(prev_file_size);
                while zero_offset < offset {
                    let n = min(buf.len() as u64, offset - zero_offset) as usize;
                    let n_written = self.write(file, zero_offset, &buf[..n])?;
                    assert!(n == n_written);
                    zero_offset += n as u64;
                }
            }
        }
        // we no longer have to worry about the silly case where offset = u32::MAX + 1
        // — no data will actually be written in that case.
        let Ok(mut offset) = u32::try_from(offset) else {
            return Ok(0);
        };
        let mut buf = &buf[..=min(buf.len() - 1, (u32::MAX - offset) as usize)];
        let mut write_count = 0;
        if offset % BLOCK_SECTOR_SIZE as u32 != 0 {
            // align to disk sector
            let offset_in_sector = offset as usize % BLOCK_SECTOR_SIZE;
            let mut sector = [0; BLOCK_SECTOR_SIZE];
            self.read_file_sector(file, offset / BLOCK_SECTOR_SIZE as u32, &mut sector)?;
            let n = min(BLOCK_SECTOR_SIZE - offset_in_sector, buf.len());
            sector[offset_in_sector..offset_in_sector + n].copy_from_slice(buf);
            self.write_file_sector(file, offset / BLOCK_SECTOR_SIZE as u32, &sector)?;
            write_count += n as u32;
            offset += n as u32;
            buf = &buf[n..];
            // this could overflow, but if it does we will have buf.len() == 0
            offset = offset.wrapping_add(write_count);
        }
        if buf.is_empty() {
            return Ok(write_count as usize);
        }
        assert_eq!(offset % BLOCK_SECTOR_SIZE as u32, 0);
        while buf.len() > BLOCK_SECTOR_SIZE {
            // write whole sectors
            self.write_file_sector(
                file,
                offset / BLOCK_SECTOR_SIZE as u32,
                &buf[..BLOCK_SECTOR_SIZE],
            )?;
            write_count += BLOCK_SECTOR_SIZE as u32;
            offset += BLOCK_SECTOR_SIZE as u32;
            buf = &buf[BLOCK_SECTOR_SIZE..];
        }
        if buf.is_empty() {
            return Ok(write_count as usize);
        }
        assert_eq!(offset % BLOCK_SECTOR_SIZE as u32, 0);
        // write final sector
        let mut sector = [0; BLOCK_SECTOR_SIZE];
        self.read_file_sector(file, offset / BLOCK_SECTOR_SIZE as u32, &mut sector)?;
        sector[..buf.len()].copy_from_slice(buf);
        self.write_file_sector(file, offset / BLOCK_SECTOR_SIZE as u32, &sector)?;
        write_count += buf.len() as u32;
        Ok(write_count as usize)
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
        self.fat.sync(&mut self.block)?;
        // TODO: sync file_info
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
        let mut root = fat.open(fat.root()).unwrap();
        let entries: Vec<OwnedDirEntry> = fat.readdir(&mut root).unwrap().to_sorted_vec();
        fn check_entry(entry: &OwnedDirEntry, name: &str, r#type: INodeType) {
            assert_eq!(&entry.name, name);
            assert_eq!(entry.r#type, r#type);
        }
        assert_eq!(entries.len(), 4);
        check_entry(&entries[0], "a", INodeType::File);
        check_entry(&entries[1], "b", INodeType::File);
        check_entry(&entries[2], "c", INodeType::File);
        check_entry(&entries[3], "d", INodeType::Directory);
        let mut dir_d = fat.open(entries[3].inode).unwrap();
        let mut file_a = fat.open(entries[0].inode).unwrap();
        let mut buf = [0; 512];
        let n = fat.read(&mut file_a, 0, &mut buf[..]).unwrap();
        assert_eq!(&buf[..n], b"file a\n");
        fat.release(file_a.inode);
        let dir_d_entries = fat.readdir(&mut dir_d).unwrap().to_sorted_vec();
        assert_eq!(dir_d_entries.len(), 1);
        check_entry(&dir_d_entries[0], "f", INodeType::File);
        let file_f_inode = dir_d_entries[0].inode;
        let mut file_f = fat.open(file_f_inode).unwrap();
        let n = fat.read(&mut file_f, 0, &mut buf[..]).unwrap();
        assert_eq!(&buf[..n], b"inner file\n");
        fat.release(file_f.inode);
        fat.release(dir_d.inode);
        fat.release(root.inode);
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
}
