#![allow(dead_code)]
#![allow(unused_variables)]
use super::vfs::*;
use crate::dev::block::{Block, BlockType};
use alloc::string::String;

#[derive(Clone)]
pub struct Tempfs {
    block: Block,
}

impl FileSystem for Tempfs {
    fn blkid(&self) -> Blkid {
        self.block.block_idx() as Blkid
    }

    fn read_ino(&self, ino: super::inode::InodeNum) -> super::inode::MemInode {
        todo!()
    }

    fn root_ino(&self) -> super::inode::InodeNum {
        todo!()
    }
    
    fn stat(&self, path: String) -> Option<super::inode::Stat> {
        todo!()
    }
    
    fn mkdir(&self, path: String) -> Option<super::inode::InodeNum> {
        todo!()
    }
    
    fn mv(&self, src_path: String, dest_path: String) -> Option<()> {
        todo!()
    }
    
    // fn cp(&mut self, src_path: String, dest_path: String) -> Option<usize> { todo!() }
    
    fn open(&self, path: String) -> Option<File> {
        todo!()
    }
    
    fn close(&self, file: &File) -> Option<()> {
        todo!()
    }
    
    fn read(&self, file: &File, buf: &[u8]) -> Option<()> {
        todo!()
    }
    
    fn write(&self, file: File, buf: &[u8]) -> Option<()> {
        todo!()
    }
    
    fn del(&self, path: String) -> Option<()> {
        todo!()
    }
    
    fn create(&self, path: String) -> Option<super::inode::InodeNum> {
        todo!()
    }
}

impl Tempfs {
    pub fn detect(block: Block) -> Option<Tempfs> {
        if matches!(block.block_type(), BlockType::Tempfs) {
            Option::Some(Tempfs { block })
        } else {
            Option::None
        }
    }
}
