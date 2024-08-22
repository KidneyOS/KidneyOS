
#![allow(dead_code)]
#![allow(unused_variables)]
use crate::dev::block::{BlockType,Block};
use super::vfs::*;

#[derive(Clone)]
pub struct Tempfs {
    block: Block,
}

impl FileSystem for Tempfs{
    fn blkid(&self) -> Blkid {
        self.block.block_idx() as Blkid
    }

    fn read_ino(&self, ino: super::inode::InodeNum) -> super::inode::MemInode {
        todo!()
    }

    fn root_ino(&self) -> super::inode::InodeNum {
        todo!()
    }
}


impl Tempfs {
    pub fn detect(block: Block) -> Option<Tempfs>{
        if matches!(block.block_type(), BlockType::Tempfs) {
            Option::Some(Tempfs{block})
        } else {
            Option::None
        }
    }

}
