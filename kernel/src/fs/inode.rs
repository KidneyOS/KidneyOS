use alloc::{string::String, vec::Vec};
use core::iter::Iterator;

#[derive(Clone)]
pub struct Stat {
    st_dev: u32,
    st_ino: u32,
    st_mode: u16,
    st_nlink: u16,
    st_uid: u32,
    st_gid: u32,
    st_rdev: u32,
    st_size: u64,
    st_blksize: u32,
    st_blocks: u64,
    st_atime: u64,
    st_mtime: u64,
    st_ctime: u64,
}

#[derive(Clone)]
pub struct MemInode {
    block: String, // superblock
    stat: Stat,
    children: Vec<u32>, //vector of children inode number
    inlining: Vec<u8>, // inline data
    name: String,
}

impl MemInode {
    // used by fs 
    pub fn create(ino: u32, name: &str, super_name: &str) {
        MemInode {
            block: super_name.into(),
            stat: Stat {
                st_dev: 0,
                st_ino: ino,
                st_mode: 0,
                st_nlink: 0,
                st_uid: 0,
                st_gid: 0,
                st_rdev: 0,
                st_size: 0,
                st_blksize: 0,
                st_blocks: 0,
                st_atime: 0,
                st_mtime: 0,
                st_ctime: 0,
            },
            children: Vec::new(),
            inlining: Vec::new(),
            name: name.into(),
        };
    }
    //pub fn link 

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.into();
    }

    pub fn get_children(&self) -> &Vec<u32> {
        &self.children
    }

    pub fn add_child(&mut self, child: u32) {
        self.children.push(child);
    }

    pub fn remove_child(&mut self, child: u32) {
        self.children.retain(|&x| x != child);
    }

    pub fn set_children(&mut self, children: Vec<u32>) {
        self.children = children;
    }

    pub fn get_inlining(&self) -> &Vec<u8> {
        &self.inlining
    }

    pub fn set_inlining(&mut self, inlining: Vec<u8>) {
        self.inlining = inlining;
    }

    pub fn get_block(&self) -> &str {
        &self.block
    }

    pub fn set_block(&mut self, block: &str) {
        self.block = block.into();
    }

    pub fn stat(&self) -> &Stat {
        &self.stat
    }

    pub fn set_stat(&mut self, stat: Stat) {
        self.stat = stat;
    }

    pub fn iter_children(&self) -> impl Iterator<Item = &u32> {
        self.children.iter()
    }
}
