use crate::dev::block::{Block, BlockType, BlockManager, BlockSector, block_init};
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

pub struct Stat {
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
    parent: Option<Box<Dentry<'a>>>,
    children: Vec<Dentry<'a>>,
    block: &'a SuperBlock
}


impl<'a> Dentry<'a> {

    pub fn create_root(block: &'a SuperBlock, ino_num: u32) -> Dentry<'a> {
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
        &self.children
    }

    pub fn revalidate(flags: u32){
        todo!();
    }
}

pub struct MemInode {
    block: Block,
    stat: Stat,
    offset: BlockSector, //up to fs implementation 
}


impl MemInode {
    pub fn create(ino: MemInode, dentry: Dentry) {
                
    }
    //pub fn link 
}



pub enum FsType {
    Tempfs(Tempfs),

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
    pub fn new(name: String, fs: FsType) -> SuperBlock {
        SuperBlock {
            name,
            fs,
        }
    }
    
    pub fn get_root(&self) -> &Dentry {
        let fs = self.fs.unwrap();
        fs.get_root()
    }
}



pub trait FileSystem {
    fn try_init(block: Block) -> Option<SuperBlock>;
    fn get_root(&mut self) -> &Dentry;
    fn lookup(&self, dentry: Dentry) -> Option<MemInode>;


}
pub struct Vfs {
    root: SuperBlock,
    registered: Vec<SuperBlock>,
    blocks: BlockManager,
}

impl Vfs {

    pub fn new(root:SuperBlock, ) {
        Vfs {
            root,
            registered: Vec::new(),
            blocks: block_init(),
        };
    }

    pub fn register_filesys(block: Block) -> Option<SuperBlock> {
        // Detect FS type
        let mut fs: Option<SuperBlock>;
        //try every fs type
        fs = Tempfs::try_init(block.clone());
        if fs.is_some() {
            return fs;
        }
        Option::None
    }

    pub fn mount_filesys(&self, block: Block) {
        todo!();
    }

    pub fn resolve_path(&self, absolute_path: &str) -> Dentry{
        match self.root.fs {
            FsType::Tempfs(fs) => {
                todo!();
            }
        }
    }

    pub fn stat(&self, path: &str) -> Result<Stat, IOError> {
        let dentry = self.resolve_path(path);
        match dentry.block.fs {
            FsType::Tempfs(fs) => {
                let mem_inode: MemInode = fs.lookup(dentry).unwrap();
                Ok(mem_inode.stat)
            }

        }
    }

}

pub struct Path {
    Vec<String> names,
}

pub fn fs_init(blocks: BlockManager, root_name: &str) -> Option<Vfs>{ 
    let root = if let Option::Some(blk) = Vfs::register_filesys(blocks.by_n)me(blocks) {
        blk
    } else{
        return Option::None;
    };
    Option::Some(Vfs {
        root: SuperBlock::new(root), 
        registered: Vec::new(),
        blocks
    })

}






