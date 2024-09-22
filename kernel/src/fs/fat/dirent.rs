use crate::fs::fat::FatFS;
use crate::vfs::{OwnedDirEntry, Result};
use zerocopy::{FromBytes, FromZeroes};

#[repr(C)]
#[derive(FromZeroes, FromBytes)]
// FAT directory entry, as stored on disk.
// NOTE: all the fields in this struct are thankfully aligned, so there are no hidden padding bytes here.
struct FatDirEntry {
    name: [u8; 11],
    attr: u8,
    _reserved: u8,
    creation_time_tenth: u8,
    creation_time: u16,
    creation_date: u16,
    access_date: u16,
    first_cluster_hi: u16,
    write_time: u16,
    write_date: u16,
    first_cluster_lo: u16,
    file_size: u32,
}

#[repr(C)]
#[derive(FromZeroes, FromBytes)]
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

/// Parses a directory entry
///
/// NOTE: the `name` member of the return value will be empty!! The name is instead stored in the `name` argument, for lifetime reasons.
pub fn parse_dir_entry(
    fs: &mut FatFS,
    data: [u8; 32],
    name: &mut String,
) -> Result<Option<OwnedDirEntry>> {
    let _ = fs;
    let _ = data;
    let _ = name;
    todo!()
}
