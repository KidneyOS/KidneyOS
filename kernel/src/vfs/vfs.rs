use crate::{dev::block::{BlockManager, Block, BlockType}, fs::inode::MemInode};
use crate::fs::tempfs;
use alloc::collections::btree_map::BTreeMap;



#[derive(Clone)]
pub enum FsType{
    Tempfs(Tempfs),
}

impl FsType {
    pub fn unwrap(&self) -> &dyn FileSystem {
        match self {
            FsType::Tempfs(fs) => fs,
        }
    }
}



pub struct SuperBlock {
     
    ino_cache: BTreeMap<u32, MemInode>,

}

pub struct Dentry {

}


pub struct Vfs {
    dentry_cache: BTreeMap<String, Dentry>,
    registered: Vec<SuperBlock>,
}
