use crate::fs::fat::{error, FatFS, fat::FatEntry};
use crate::vfs::{OwnedDirEntry, Result, DirectoryIterator, DirEntry, INodeNum};
use crate::block::block_core::BLOCK_SECTOR_SIZE;
use zerocopy::{FromBytes, FromZeroes, Unaligned};
use alloc::collections::BTreeMap;
use zerocopy::little_endian::{U16, U32};

#[repr(C)]
#[derive(FromZeroes, FromBytes, Unaligned)]
struct FatDirEntry {
    name: [u8; 11],
    attr: u8,
    _reserved: u8,
    creation_time_tenth: u8,
    creation_time: U16,
    creation_date: U16,
    access_date: U16,
    first_cluster_hi: U16,
    write_time: U16,
    write_date: U16,
    first_cluster_lo: U16,
    file_size: U32,
}

#[repr(C)]
#[derive(FromZeroes, FromBytes, Unaligned)]
struct FatDirEntryLongName {
    ord: u8,
    name1: [u8; 10],
    attr: u8,
    _unused1: u8,
    _chksum: u8,
    name2: [u8; 12],
    _unused2: [u8; 2],
    name3: [u8; 4],
}

pub struct Directory {
    entries: BTreeMap<u64, OwnedDirEntry>,
    lookup: BTreeMap<String, u64>,
    id: u64,
}

const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN: u8 = 0x02;
const ATTR_SYSTEM: u8 = 0x04;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_ARCHIVE: u8 = 0x20;
const ATTR_LONG_NAME: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;

impl Directory {
    fn read_one_entry(&mut self, bytes: &[u8], long_name: &mut String) -> Result<()> {
        let entry: &FatDirEntry = FatDirEntry::ref_from(bytes).unwrap();
        if entry.attr == ATTR_LONG_NAME {
            // a "long name" entry (stores part of a file name)
            let entry: &FatDirEntryLongName = FatDirEntryLongName::ref_from(bytes).unwrap();
            let mut utf16 = [0u16; 13];
            let mut it = utf16.iter_mut();
            for c in entry.name1.chunks(2) {
                *it.next().unwrap() = u16::from_le_bytes([c[0], c[1]]);
            }
            for c in entry.name2.chunks(2) {
                *it.next().unwrap() = u16::from_le_bytes([c[0], c[1]]);
            }
            for c in entry.name3.chunks(2) {
                *it.next().unwrap() = u16::from_le_bytes([c[0], c[1]]);
            }
            let mut utf8 = [0; 39];
            let mut i = 0;
            let length = utf16.iter().position(|&x| x == 0).unwrap_or(utf16.len());
            for c in char::decode_utf16(utf16[..length].iter().copied()) {
                let Ok(c) = c else {
                    return error!("file name contains bad UTF-16.");
                };
                i += c.encode_utf8(&mut utf8[i..i + 4]).len();
            }
        } else if entry.name[0] == 0 || entry.name[0] == 0xE5 {
            // this entry is free.
            return Ok(());
        } else {
            // ordinary directory entry
            todo!()
        }
        Ok(())
    }
    fn read_from_disk_sector(&mut self, fs: &mut FatFS, sector: u32, long_name: &mut String) -> Result<()> {
        let mut data = [0; BLOCK_SECTOR_SIZE];
        fs.block.read(sector, &mut data)?;
        for i in 0..BLOCK_SECTOR_SIZE / 32 {
            self.read_one_entry(&data[32 * i..32 * (i + 1)], long_name)?;
        }
        Ok(())
    }
    pub fn read(fs: &mut FatFS, inode: INodeNum) -> Result<Self> {
        let mut cluster = inode;
        let mut dir = Directory {
            entries: BTreeMap::new(),
            lookup: BTreeMap::new(),
            id: 0,
        };
        let mut long_name = String::new();
        if inode == 0 {
            // root directory is special in FAT-16
            // (note: root inode will not be 0 for FAT-32)
            for disk_sector in fs.fat16_root_disk_sectors() {
                dir.read_from_disk_sector(fs, disk_sector, &mut long_name)?;
            }
        } else {
            loop {
                for disk_sector in fs.disk_sectors_in_cluster(cluster) {
                    dir.read_from_disk_sector(fs, disk_sector, &mut long_name)?;
                }
                match fs.fat.entry(cluster) {
                    FatEntry::Defective | FatEntry::Free => {
                        return error!("cluster {cluster} is referenced but not allocated.");
                    }
                    FatEntry::HasNext(next) => {
                        cluster = next;
                    }
                    FatEntry::Eof => break,
                }
            }
        }
        Ok(dir)
    }
    pub(super) fn lookup(&self, name: &str) -> Option<INodeNum> {
        let id = self.lookup.get(name)?;
        Some(self.entries[&id].inode)
    }
}

pub struct FatDirectoryIterator<'a> {
    dir: Result<&'a Directory>,
    offset: u64,
}

impl<'a> FatDirectoryIterator<'a> {
    pub fn new(dir: Result<&'a Directory>, offset: u64) -> Self {
        Self {
            dir,
            offset
        }
    }
}

impl DirectoryIterator for FatDirectoryIterator<'_> {
    fn next(&mut self) -> Result<Option<DirEntry<'_>>> {
        let dir = self.dir.as_ref().map_err(|e| e.clone())?;
        Ok(dir.entries.range(self.offset..).next().map(|(_, entry)| entry.to_borrowed()))
    }
    fn offset(&self) -> u64 {
        self.offset
    }
}
