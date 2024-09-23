use crate::block::block_core::BLOCK_SECTOR_SIZE;
use crate::fs::fat::{error, fat::FatEntry, FatFS};
use crate::vfs::{FileInfo, INodeNum, INodeType, Result};
use zerocopy::little_endian::{U16, U32};
use zerocopy::{FromBytes, FromZeroes, Unaligned};

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

const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN: u8 = 0x02;
const ATTR_SYSTEM: u8 = 0x04;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_DIRECTORY: u8 = 0x10;
#[allow(dead_code)] // TODO: delete once we have FAT writing (which should set this attribute)
const ATTR_ARCHIVE: u8 = 0x20;
const ATTR_LONG_NAME: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;

pub struct DirEntry {
    pub name: usize,
    pub info: FileInfo,
}

pub struct Directory {
    pub entries: Vec<DirEntry>,
    pub names: Vec<u8>,
    pub prev_name_end: usize,
}

impl Directory {
    fn read_one_entry(&mut self, bytes: &[u8]) -> Result<()> {
        let entry: &FatDirEntry = FatDirEntry::ref_from(bytes).unwrap();
        let attr = entry.attr;
        if attr == ATTR_LONG_NAME {
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
            let mut utf8_len = 0;
            let length = utf16.iter().position(|&x| x == 0).unwrap_or(utf16.len());
            for c in char::decode_utf16(utf16[..length].iter().copied()) {
                let Ok(c) = c else {
                    return error!("file name contains bad UTF-16.");
                };
                utf8_len += c.encode_utf8(&mut utf8[utf8_len..utf8_len + 4]).len();
            }
            // Oddly, the "long name" entries are stored in reverse.
            // So we reverse each entry, then reverse the whole thing at the end.
            self.names.extend(utf8[..utf8_len].iter().copied().rev());
        } else if entry.name[0] == 0 || entry.name[0] == 0xE5 {
            // this entry is free.
            return Ok(());
        } else {
            // ordinary directory entry
            let name = self.prev_name_end;
            if name < self.names.len() {
                // account for fact that long name entries are stored in reverse
                self.names[name..].reverse();
            } else {
                // no long name — read short name
                fn read_short_name_part(name: &mut Vec<u8>, mut part: &[u8]) -> Result<()> {
                    // remove trailing spaces
                    while part.last().is_some_and(|&c| c == b' ') {
                        part = &part[..part.len() - 1];
                    }
                    for c in part.iter().copied() {
                        let c = match c {
                            // technically 5 can replace E5 in the KANJI encoding
                            5 => char::from(0xE5u8),
                            b'a'..=b'z'
                            | 0..=4
                            | 6..=0x20
                            | 0x22
                            | 0x2A
                            | 0x2B
                            | 0x2C
                            | 0x2E
                            | 0x2F
                            | 0x3A
                            | 0x3B
                            | 0x3C
                            | 0x3D
                            | 0x3E
                            | 0x3F
                            | 0x5B
                            | 0x5C
                            | 0x5D
                            | 0x7C => {
                                return error!("invalid character in FAT short name: {c}");
                            }
                            // Strictly speaking this isn't correct for x >= 128,
                            // and we should instead refer to the "OEM character set".
                            // Doesn't really matter since long names are standard now.
                            x => char::from(x),
                        };
                        let mut utf8 = [0; 4];
                        let n = c.encode_utf8(&mut utf8[..]).len();
                        name.extend_from_slice(&utf8[..n]);
                    }
                    Ok(())
                }
                // Linux stores directory entries for . and ..
                // This seems to be against the spec, since 0x2e == '.' is not
                // allowed in short file names.
                if &entry.name == b".          " || &entry.name == b"..         " {
                    return Ok(());
                }
                if &entry.name == b"           " {
                    return error!("empty file name");
                }
                read_short_name_part(&mut self.names, &entry.name[..8])?;
                if &entry.name[8..] != b"   " {
                    self.names.push(b'.');
                    read_short_name_part(&mut self.names, &entry.name[8..])?;
                }
            }
            let r#type = if (attr & ATTR_DIRECTORY) != 0 {
                INodeType::Directory
            } else {
                INodeType::File
            };
            let size: u64 = entry.file_size.into();
            let cluster =
                u32::from(entry.first_cluster_lo) | u32::from(entry.first_cluster_hi) << 16;
            let info = FileInfo {
                r#type,
                inode: cluster,
                size,
                nlink: 1,
            };
            self.names.push(0);
            self.prev_name_end = self.names.len();
            self.entries.push(DirEntry { name, info })
        }
        Ok(())
    }
    fn read_from_disk_sector(&mut self, fs: &mut FatFS, sector: u32) -> Result<()> {
        let mut data = [0; BLOCK_SECTOR_SIZE];
        fs.block.read(sector, &mut data)?;
        for i in 0..BLOCK_SECTOR_SIZE / 32 {
            self.read_one_entry(&data[32 * i..32 * (i + 1)])?;
        }
        Ok(())
    }
    pub fn read(fs: &mut FatFS, inode: INodeNum) -> Result<Self> {
        let mut cluster = inode;
        let mut dir = Directory {
            entries: vec![],
            names: vec![],
            prev_name_end: 0,
        };
        if inode == 0 {
            // root directory is special in FAT-16
            // (note: root inode will not be 0 for FAT-32)
            for disk_sector in fs.fat16_root_disk_sectors() {
                dir.read_from_disk_sector(fs, disk_sector)?;
            }
        } else {
            loop {
                for disk_sector in fs.disk_sectors_in_cluster(cluster) {
                    dir.read_from_disk_sector(fs, disk_sector)?;
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
}
