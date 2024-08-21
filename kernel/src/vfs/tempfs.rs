use super::{vfs::{File, Vfs, Dentry}, inode::{MemInode, Stat}, superblock::{SuperBlock, FileSystem, FsType}};
use crate::dev::block::{BlockType,Block};
use alloc::sync::Arc;

#[derive(Clone)]
pub struct Tempfs {
    block: Block,
}

impl FileSystem for Tempfs{
    fn try_init(block: Block) -> Option<SuperBlock>{
        if let BlockType::BlockTempfs = block.block_type() {
            Option::Some(
                SuperBlock::new(FsType::Tempfs(Tempfs{
                    block,
                }))
            )
        } else {
            Option::None
        }
    }

    fn device_name(&self) -> &str {
        self.block.block_name()
    }

    fn get_root_ino(&self) -> u32 {
        0
    }

    fn read_inode(&self, ino: u32) -> Option<MemInode>  {
        todo!()
    }

    fn open(&self, path: &str) -> Option<File> { todo!() }
    fn close(&self, file: &File) -> bool { todo!() }
    fn read(&self, file: &File, buffer: &mut [u8]) -> u32 { todo!() }
    fn write(&self, file: &File, buffer: &[u8]) -> u32 { todo!() }
    fn create(&mut self, path: &str, name: &str) -> u32 { todo!() }
    fn delete(&self, path: &str) -> bool { todo!() }
    fn mkdir(&mut self, path: &str, name: &str) -> u32 { todo!() }
    fn rmdir(&mut self, path: &str, name: &str) -> bool { todo!() }
    fn cp(&self, path: &str, name: &str) -> u32 {
        todo!()
    }
    fn mv(&self, path: &str, name: &str) -> bool {
        todo!()
    }
}
