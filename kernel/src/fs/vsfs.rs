use crate::fs::vfs::FileSystem;
use crate::fs::vfs::File;
use crate::dev::block::Block;

#[repr(c, packed)];
struct Inode {
}



struct VSFS<'a> {
    block: &'a Block,
}


impl<'a> FileSystem for VSFS<'a> {
    
    fn new(block: &'a Block) -> Self {
        VSFS {
            block
        }
    }

    fn open(&self, path: &str) -> Option<File> {
        todo!()
    }

    fn close(&self, file: &File) -> bool {
        todo!()
    }

    fn read(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32 {
        todo!()
    }

    fn write(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32 {
        todo!()
    }

    fn create(&mut self, path: &str, name: &str) -> bool {
        todo!()
    }

    fn delete(&self, path: &str) -> bool {
        todo!()
    }

    fn list_dir(&self, path: &str) -> Option<Vec<String>> {
        todo!()
    }

    fn mkdir(&mut self, path: &str, name: &str) -> bool {
        todo!()
    }

    fn rmdir(&mut self, path: &str, name: &str) -> bool {
        todo!()
    }
}

