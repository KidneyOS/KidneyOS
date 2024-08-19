use crate::fs::vfs::{FileSystem, File, Vfs, SuperBlock, MemInode, Dentry, FsType};
use crate::dev::block::{BlockType,Block};



pub struct Tempfs  {
    block: Block,
    root: Option<Dentry>,
};



impl FileSystem for Tempfs {
    pub fn try_init(block: Block) -> Option<SuperBlock>{
        if let block.block_type() == BlockType::BlockTempfs {
            Option::Some(SuperBlock {
                name: block.block_name(),
                fs: FsType::Tempfs( Tempfs {
                    block
                }),     
            })
        }else {
            Option::None
        }
    }
    
    pub fn get_root(&mut self) -> &Dentry {
        if let self.root == Option::None {
            self.root = Dentry::create_root(self.block, 0)
        }
        &self.root.unwrap()
    }




}






    







// impl<'a> FileSystem for Tempfs<'a> {
//     fn new(block: &Block) -> Self {
//         tempfs {
//             block
//         }
//     }
//
//     fn open(&self, path: &str) -> Option<File> {
//         todo!()
//     }
//
//     fn close(&self, file: &File) -> bool {
//         todo!()
//     }
//
//     fn read(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32 {
//         todo!()
//     }
//
//     fn write(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32 {
//         todo!()
//     }
//
//     fn create(&mut self, path: &str, name: &str) -> bool {
//         todo!()
//     }
//
//     fn delete(&self, path: &str) -> bool {
//         todo!()
//     }
//
//     fn list_dir(&self, path: &str) -> Option<Vec<String>> {
//         todo!()
//     }
//
//     fn mkdir(&mut self, path: &str, name: &str) -> bool {
//         todo!()
//     }
//
//     fn rmdir(&mut self, path: &str, name: &str) -> bool {
//         todo!()
//     }
// }
