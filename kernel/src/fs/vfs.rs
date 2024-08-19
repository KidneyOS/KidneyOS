use crate::dev::block::{Block, BlockType, BlockManager};
use alloc::{vec::Vec, string::String};
use crate::sync::irq::MutexIrq;
use crate::fs::{ext2, vsfs, fat, tempfs::Tempfs};
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

struct Stat {
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
    mounted: bool,
    parent: Option<Box<Dentry>>,
    children: Vec<Dentry>,
    block: &'a SuperBlock
}


impl <'a>Dentry {

    pub fn create_root(block: &'a SuperBlock, ino_num: u32) -> Dentry {
        Dentry {
            count: 1,
            ino_number: ino_num,
            name: "/".into(),
            mounted: false,
            parent: Option::None,
            children: Vec::new(),
            block
        }
    }

    pub fn forward(&self) -> &[Dentry] {
        todo!();
    }


    pub fn revalidate(u32: flags){
        todo!();
    }
}

pub struct MemInode {
    block: Block,
    stat: stat,
    offset: 
}

impl MemInode {
    
    pub fn create(ino: MemInode, Dentry: Dentry) {
                
    }
    //pub fn link 
}



pub enum FsType {
    Tempfs(Tempfs)
}

impl FsType {
    pub fn unwrap(&self) -> &impl FileSystem {
        match self {
            FsType::Tempfs(fs) => fs,
        }
    }
}


pub struct SuperBlock {
    name: String,
    fs: FsType,
}


impl SuperBlock {

    pub fn try_init(block: Block) -> Option<SuperBlock> {
        // Detect FS type
        let mut fs: Option<SuperBlock>;

        //try every fs type
        fs = Tempfs::try_init(block.clone())
        if let fs == Option::Some(a) {
            return fs;
        }
        Option::None
    }
    
    pub fn get_root(&self) -> &Dentry {
        let fs = self.fs.unwrap();
        fs.get_root()
    }
}

pub trait FileSystem {
    fn try_init(block: Block) -> Option<SuperBlock>;
    fn get_root(&mut self) -> &Dentry;



}
pub struct Vfs {
    root: SuperBlock,
    registered: Vec<SuperBlock>,
    blocks: BlockManager,
}

impl Vfs {

    pub fn register_filesys(&self, dev_name: &str ) -> SuperBlock {
        todo!();
    }

    pub fn mount_filesys(&self, dev_name: &str) {
        todo!();
    }

    pub fn resolve_path(&self, absolute_path: &str) -> Dirent{
        todo!();
    }

    pub fn stat(&self, path: &str) {
        todo!();
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






