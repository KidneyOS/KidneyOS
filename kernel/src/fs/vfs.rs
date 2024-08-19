use crate::dev::block::{Block, BlockType};
use alloc::{vec::Vec, string::String};
use crate::fs::{ext2, vsfs, fat};
use core::error::Error;
use core::fmt;
use core::fmt::Debug;



pub struct IOError {
    message: String,
}

impl fmt::Display for IOError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IO error: {}", self.message)
    }
}

impl Debug for IOError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IO error: {}", self.message)
    }
}

impl Error for IOError {
    fn source(&self) -> Option<&(dyn Error + 'static)> { 
        Some(&self.message)
    }
}

impl IOError {
    pub fn new(message: String) -> Self {
        IOError {
            message,
        }
    }
}

struct Dentry {
    inode: u32,
    name: String,
    file_type: FileType,
}

enum FileType {
    Directory,
    File,
    Link,
}

impl Dentry {
    pub fn new(inode: u32, name: String, file_type: FileType) -> Self {
        Dentry {
            inode,
            name,
            file_type,
        }
    }
    pub fn alphasort(a: &Dentry, b: &Dentry) -> bool {
        a.name < b.name
    }
    pub fn closedir(&self) {
        todo!()
    }
    pub fn dirfd(&self) {
        todo!()
    }
    pub fn fdopendir(&self) {
        todo!()
    }
    pub fn opendir(&self) {
        todo!()
    }
    pub fn readdir(&self) {
        todo!()
    }
    pub fn readdir_r(&self) {
        todo!()
    }
    pub fn rewinddir(&self) {
        todo!()
    }
    pub fn scandir(&self) {
        todo!()
    }
    pub fn seekdir(&self) {
        todo!()
    }
    pub fn telldir(&self) {
        todo!()
    }
}

struct MemInode {
    block: Block,
    stat: stat,
}

impl MemInode {
    
    pub fn create(ino: MemInode, Dentry: Dentry) {
        


    }
    //pub fn link 
}

struct SuperBlock {
    root: Dentry,
    device: Block,


}
    
struct Fstype {
    name: String,
    fs_flags: u32,
    block: Block,
}

struct stat {
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

pub struct File {
    mem_inode: MemInode,
    pathname: String,
}
pub trait FileSystem {
    fn new(block: &Block) -> Self;
    // fn mount(&mut self);
    // fn unmount(&mut self);
    fn open(&self, path: &str) -> Option<File>;
    fn close(&self, file: &File) -> bool;
    fn read(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32;
    fn write(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32;
    fn create(&mut self, path: &str, name: &str) -> bool;
    fn delete(&self, path: &str) -> bool;
    fn list_dir(&self, path: &str) -> Option<Vec<String>>;
    fn mkdir(&mut self, path: &str, name: &str) -> bool;
    fn rmdir(&mut self, path: &str, name: &str) -> bool;
}
pub struct Vfs {
}

impl Vfs {
    pub fn mount(block: &Block) {
        let fs_type = block.block_type();
        match fs_type {
            BlockType::BlockTempfs => {
                
            }
            _ => {
                panic!("Unformatted block device");
            }
        }
    }

    pub fn open(&self, path: &str) -> Option<File> {
        todo!()
    }

    
}

