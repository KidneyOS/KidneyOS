use crate::dev::block::{Block, BlockManager, block_init};
use alloc::{vec::Vec, string::String};
use crate::sync::irq::{MutexIrq};
use crate::fs::{inode::{MemInode, Stat}, superblock::{SuperBlock, FsType, FileSystem}, tempfs::Tempfs};
use core::error::Error;
use core::fmt;
use core::fmt::Debug;
use alloc::collections::btree_map::BTreeMap;


// TODO: add lookup_inode() function with inode cache and lookup inode if not in cache
// TODO: clear inode cache after a while
// TODO: change BtreeMap to store Arc<MemInode>
pub trait FileSystem {
    fn try_init(block: Block) -> Option<SuperBlock>;
    fn get_root_ino(&self) -> u32;
    fn read_inode(&self, inode: u32) -> Option<MemInode>;
    fn device_name(&self) -> &str;
    fn open(&self, path: &str) -> Option<File>;
    fn close(&self, file: &File) -> bool;
    fn read(&self, file: &File, amount: u32) -> u32;
    fn write(&self, file: &File, amount: u32) -> u32;
    fn create(&mut self, path: &str, name: &str) -> u32;
    fn delete(&self, path: &str) -> bool;
    fn mkdir(&mut self, path: &str, name: &str) -> u32;
    fn rmdir(&mut self, path: &str, name: &str) -> bool;
    fn cp(&self, path: &str, name: &str) -> u32;
    fn mv(&self, path: &str, name: &str) -> bool;
}
#[derive(Clone)]
pub enum FsType{
    Tempfs(Tempfs),
}

impl FsType {
    pub fn unwrap(&self) -> &impl FileSystem {
        match self {
            FsType::Tempfs(fs) => fs,
        }
    }
}


pub struct SuperBlock{
    // name: String,
    fs: FsType,
    root: MutexIrq<Dentry>,
    btree: BTreeMap<u32, MemInode>,
}


impl SuperBlock {
    pub fn new(fs: FsType) -> SuperBlock {
        SuperBlock {
            fs: fs.clone(),
            root: MutexIrq::new(
                Dentry::create_root(fs.unwrap().device_name(), 
                 fs.unwrap().get_root_ino()
                )
            ),
            btree: BTreeMap::new(),
        }
    }
    pub fn get_root(&self) -> &MutexIrq<Dentry> {
        &self.root
    }

    pub fn get_fs(&self) -> &impl FileSystem {
        self.fs.unwrap()
    }

    pub fn fs_name(&self) -> &str {
        self.fs.unwrap().device_name()
    }

    pub fn lookup_inode(&self, ino: u32) -> Option<MemInode> {
        if let Option::Some(&inode) = self.btree.get(&ino) {
            return Option::Some(inode.clone());
        }
        let inode = self.fs.unwrap().read_inode(ino);
        if inode.is_none() {
            return Option::None;
        }
        self.btree.insert(ino, inode.clone().unwrap());
        Option::Some(inode.unwrap())
    }
}