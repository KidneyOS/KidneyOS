#![allow(unused_variables)]
#![allow(dead_code)]
use super::vfs::*;
use alloc::{string::String, vec::Vec};

pub type InodeNum = u32;
pub struct Stat {
    link_count: usize,
}

impl Clone for Stat {
    fn clone(&self) -> Self {
        Stat {
            link_count: self.link_count,
        }
    }
}

pub enum InodeData {
    File,
    Directory(Vec<InodeNum>),
    Link,
    Unallocated,
}

pub struct MemInode {
    ino: u32,
    name: String,
    data: InodeData,
    blkid: Blkid,
    stat: Stat,
    dirty: bool,
}

impl MemInode {
    pub fn read_data(&self) -> &InodeData {
        &self.data
    }

    pub fn is_directory(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match &self.data {
            InodeData::Directory(v) => true,
            _ => false,
        }
    }

    pub fn get_disk_children(&self) -> Option<impl Iterator<Item = &InodeNum>> {
        #[allow(clippy::match_like_matches_macro)]
        match &self.data {
            InodeData::Directory(v) => Option::Some(v.iter()),
            _ => Option::None,
        }
    }

    pub fn ino(&self) -> InodeNum {
        self.ino
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn blkid(&self) -> Blkid {
        self.blkid
    }

    pub fn stat(&self) -> &Stat {
        &self.stat
    }
}
