// https://wiki.osdev.org/Paging
// https://wiki.osdev.org/Setting_Up_Paging

// Avoids lots of warnings about casting usize to u32 which cannot result in
// truncation on a 32-bit platform, which is all we support. It would be nice if
// you could tell clippy that you were only dealing with 32-bit usizes...
#![allow(clippy::cast_possible_truncation)]

use crate::mem::PAGE_FRAME_SIZE;
use arbitrary_int::{u10, u12, u20};
use bitbybit::bitfield;
use core::{
    mem::size_of,
    ops::{Deref, DerefMut},
};

const PAGE_DIRECTORY_LEN: usize = PAGE_FRAME_SIZE / size_of::<PageDirectoryEntry>();

#[repr(align(4096))]
pub struct PageDirectory(pub [PageDirectoryEntry; PAGE_DIRECTORY_LEN]);

impl PageDirectory {
    pub const DEFAULT: Self = Self([PageDirectoryEntry::DEFAULT; PAGE_DIRECTORY_LEN]);
}

impl Default for PageDirectory {
    fn default() -> Self {
        Self([PageDirectoryEntry::default(); PAGE_DIRECTORY_LEN])
    }
}

impl Deref for PageDirectory {
    type Target = [PageDirectoryEntry; PAGE_DIRECTORY_LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PageDirectory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[bitfield(u32, default = 0)]
pub struct PageDirectoryEntry {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    read_write: bool,
    #[bit(2, rw)]
    user_supervisor: bool,
    #[bit(3, rw)]
    write_through: bool,
    #[bit(4, rw)]
    cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(7, rw)]
    page_size: bool,
    #[bits(12..=31, rw)]
    page_table_address: u20,
}

const PAGE_TABLE_LEN: usize = PAGE_FRAME_SIZE / size_of::<PageTableEntry>();

#[repr(align(4096))]
pub struct PageTable(pub [PageTableEntry; PAGE_TABLE_LEN]);

impl Default for PageTable {
    fn default() -> Self {
        Self([PageTableEntry::default(); PAGE_TABLE_LEN])
    }
}

impl Deref for PageTable {
    type Target = [PageTableEntry; PAGE_TABLE_LEN];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[bitfield(u32, default = 0)]
pub struct PageTableEntry {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    read_write: bool,
    #[bit(2, rw)]
    user_supervisor: bool,
    #[bit(3, rw)]
    write_through: bool,
    #[bit(4, rw)]
    cache_disable: bool,
    #[bit(5, rw)]
    accessed: bool,
    #[bit(6, rw)]
    dirty: bool,
    #[bit(7, rw)]
    page_attribute_table: bool,
    #[bit(8, rw)]
    global: bool,
    #[bits(12..=31, rw)]
    page_frame_address: u20,
}

#[bitfield(u32)]
pub struct VirtualAddress {
    #[bits(22..=31, r)]
    page_directory_index: u10,
    #[bits(12..=21, r)]
    page_table_index: u10,
    #[bits(0..=11, r)]
    offset: u12,
}
