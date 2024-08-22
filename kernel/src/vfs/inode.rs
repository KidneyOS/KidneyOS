use super::vfs::*;
use alloc::{string::String, vec::Vec};

pub type InodeNum = u32;
pub struct Stat {
    link_count: usize,
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

    pub fn blkid(&self) -> Blkid {
        self.blkid
    }

}
