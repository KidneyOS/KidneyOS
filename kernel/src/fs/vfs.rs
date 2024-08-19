use crate::dev::block::{Block, BlockType, BlockManager};
use alloc::{vec::Vec, string::String};
use core::map::HashMap;
use crate::sync::irq::MutexIrq;
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
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> { 
        Option::None
    }
}

impl IOError {
    pub fn new(message: String) -> Self {
        IOError {
            message,
        }
    }
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

pub enum FileType {
    Directory,
    File,
    Link,
}

pub struct Dentry<'a> {
    count: u32, /* use count */
    ino_number: u32, /* associated inode */
    name: String, 
    mounted:bool,
    parent: &'a Dentry,
    children: Vec<Dentry>,
    block: &'a SuperBlock
}


impl <'a>Dentry {

    pub fn forward(&self) -> &[Dentry] {
        let inode = self.blockread_inode(&self)

    }


    pub fn revalidate(u32: flags){
        todo!();
    }
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

pub struct MemInode {
    block: Block,
    stat: stat,
}

impl MemInode {
    
    pub fn create(ino: MemInode, Dentry: Dentry) {
                
    }
    //pub fn link 
}



pub struct FsType {
}


pub struct SuperBlock {
    root: Dentry,
    name: String,
    ino_cache: Vec<MemInode> 
}


impl SuperBlock {
    pub fn new(block: Block) -> Option<SuperBlock> {
        // Detect FS type
        match fs_type {
            BlockType::BlockTempfs =>  
            _ => 
        }
    }


    pub fn read_inode(&self, d: Dentry) -> MemInode {
    }
    
    pub fn get_root(&self) -> &Dentry {
        &self.root
    }
}

pub trait FileSystem {
    fn new(block: &Block) -> Self;


    fn read_superblock(block: &Block)
    // fn mount(&mut self);
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
    root: SuperBlock,
    registered: Vec<SuperBlock>,
    blocks: BlockManager,

}

impl Vfs {

    pub fn register_filesys(&self, dev_name: &str ) {

    }

    pub fn mount_filesys(&self, dev_name: &str) {

    }

    pub fn resolve_path(&self, absolute_path: &str) -> Dirent{
        root = self.root.get_root();


    }

    pub fn stat(&self, path: &str) {

    }

}

pub struct Path {
    Vec<String> names,
}

pub fn fs_init(blocks: BlockManager, root: &str) -> Option<Vfs>{ 
    let root = if let SuperBlock::new(blocks.by_name(root)) == Option::Some(blk) {
        return Option::None
    } else{
        blk
    }
    Option::Some(Vfs {
        root: SuperBlock::new(blocks.by_name(root))         
        registered: Vec::new()
        blocks
    })

}






