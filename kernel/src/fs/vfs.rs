use crate::dev::block::{Block, BlockManager, block_init};
use alloc::{vec::Vec, string::String};
use crate::sync::irq::{MutexIrq};
use crate::fs::{inode::{MemInode, Stat}, superblock::{SuperBlock, FsType, FileSystem}, tempfs::Tempfs};
use core::error::Error;
use core::fmt;
use core::fmt::Debug;
use alloc::collections::btree_map::BTreeMap;


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

pub struct File {
    mem_inode: MemInode,
    buffer: Vec<u8>,
}

pub enum FileType {
    Directory,
    File,
    Link,
}

//TODO: Refactor Dentry so that parent is just u32 instead of Option<u32>. (parent of / is /)
//TODO: Make it so Dentry does not own its children, instead children should be inode numbers 
//  that we look up from the superblock in a BtreeMap<MutexIrq<Dentry>> from InodeNumber

pub struct Dentry {
    ino_number: u32, /* associated inode */
    name: String, 
    mounted: bool,
    parent: Option<u32>,
    children: Vec<MutexIrq<Dentry>>,
    super_name: String,
}

impl Dentry {
    pub fn create_root(block_name: &str, ino_num: u32) -> Dentry {
        Dentry {
            ino_number: ino_num,
            name: "/".into(),
            mounted: true,
            parent: Option::None,
            children: Vec::new(),
            super_name: block_name.into(),
        }
    }

    pub fn get_ino(&self) -> u32 {
        self.ino_number
    }
}


pub struct Vfs {
    root: SuperBlock,
    registered: Vec<SuperBlock>,
    blocks: BlockManager,
}

impl Vfs {
    pub fn new(root: SuperBlock, all_blocks: BlockManager) {
        Vfs {
            root,
            registered: Vec::new(),
            blocks: all_blocks,
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

    pub fn resolve_path(&self, absolute_path: &str) -> Result<MemInode, IOError>{
        let mut path = absolute_path.split("/");
        let mut mem_inode = Option::None;
        let mut dentry = &mut self.root.get_root().lock();
        for name in path {
            for child in self.forward(dentry) {
                let mut child = child.lock();
                if child.name == name {
                    dentry = &mut child;
                    let ino = child.get_ino();
                    let superblock = self.get_superblock(&child.super_name);
                    if superblock.is_err() {
                        return Err(IOError::new("Superblock not found".into()));
                    }

                    mem_inode = superblock.unwrap().lookup_inode(ino);
                    if mem_inode.is_none() {
                        return Err(IOError::new("Path not found".into()));
                    }
                    continue;
                }
                return Err( IOError::new("Path not found".into()))
            }
        }
        if mem_inode.is_none() {
            return Err(IOError::new("Path not found".into()));
        }
        Ok(mem_inode.unwrap())
    }

    pub fn stat(&self, path: &str) -> Result<&Stat, IOError> {
        let mem_inode = self.resolve_path(path);
        if mem_inode.is_err() {
            return Err(IOError::new("Path not found".into()));
        }
        Ok(mem_inode.unwrap().stat())
    }

    pub fn get_superblock(&self, name: &str) -> Result<&SuperBlock, IOError> {
        for superblock in self.registered.iter() {
            if superblock.fs_name() == name {
                return Ok(superblock);
            }
        }
        Err(IOError::new("Superblock not found".into()))
    }

    pub fn forward(&self, dentry: &Dentry) -> &[MutexIrq<Dentry>] {
        let fs = self.get_superblock(&dentry.super_name).unwrap().get_fs(); 
        let mem_inode = fs.read_inode(dentry.get_ino());
        for inode in mem_inode.unwrap().children {
            let child = fs.read_inode(inode);
            if child.is_none() {
                continue;
            }
            let child_dentry = Dentry {
                ino_number: inode,
                name: child.unwrap().name,
                mounted: false,
                parent: Option::Some(dentry.get_ino()),
                children: Vec::new(),
                super_name: dentry.super_name,
            };
            dentry.children.push(MutexIrq::new(child_dentry));
        }
        &dentry.children
    }

    pub fn open(&self, path: &str) -> Option<File> {
        let mem_inode = self.resolve_path(path);
        if mem_inode.is_err() {
            return Option::None;
        }

        let fs = self.get_superblock(&mem_inode.unwrap().get_block()).unwrap().get_fs();
        fs.open(path)
    }

    pub fn close(&self, file: &File) -> bool {
        let fs = self.get_superblock(&file.mem_inode.get_block()).unwrap().get_fs();
        fs.close(file)
    }

    pub fn read(&self, file: &File, amount: u32) -> u32 {
        let fs = self.get_superblock(&file.mem_inode.get_block()).unwrap().get_fs(); 
        fs.read(file, amount)
    }

    pub fn write(&self, file: &File, amount: u32) -> u32 {
        let fs = self.get_superblock(&file.mem_inode.get_block()).unwrap().get_fs();
        fs.write(file, amount)
    }

    pub fn create(&mut self, path: &str, name: &str) -> u32 {
        let mem_inode = self.resolve_path(path);
        if mem_inode.is_err() {
            return false;
        }

        let fs = self.get_superblock(&mem_inode.unwrap().get_block()).unwrap().get_fs();
        fs.create(&mem_inode.unwrap().get_name(), name)
    }
    
    pub fn delete(&self, path: &str) -> bool {
        let mem_inode = self.resolve_path(path);
        if mem_inode.is_err() {
            return false;
        }

        let fs = self.get_superblock(&mem_inode.unwrap().get_block()).unwrap().get_fs();
        fs.delete(&mem_inode.unwrap().get_name())
    }

    pub fn mkdir(&mut self, path: &str, name: &str) -> u32 {
        let mem_inode = self.resolve_path(path);
        if mem_inode.is_err() {
            return false;
        }

        let fs = self.get_superblock(&mem_inode.unwrap().get_block()).unwrap().get_fs();
        fs.mkdir(&mem_inode.unwrap().get_name(), name)
    }

    pub fn rmdir(&mut self, path: &str, name: &str) -> bool {
        // should drop dirent, remove from btree
        let mem_inode = self.resolve_path(path);
        if mem_inode.is_err() {
            return false;
        }

        let fs = self.get_superblock(&mem_inode.unwrap().get_block()).unwrap().get_fs();
        fs.rmdir(&mem_inode.unwrap().get_name(), name)
    }

    pub fn cp(&self, path: &str, new_path: &str) -> Result<u32, IOError> {
        let mem_inode = self.resolve_path(path);
        if mem_inode.is_err() {
            return Option::None;
        }

        let mut dentry = &mut self.root.get_root().lock();
        let mut parent: Option<u32> = Option::None;
        let name: String = String::new();
        for step in path {
            for child in self.forward(dentry) {
                let mut child = child.lock();
                if child.name == step {
                    parent = Option::Some(dentry.get_ino());
                    dentry = &mut child;
                    if dentry.mounted {
                        name.clear();
                    }
                    else {
                        name.push("/");
                        name.push_str(step);
                    }
                    continue;
                }
                return Err( IOError::new("Path not found".into()))
            }
        }
        

        let fs = self.get_superblock(&mem_inode.unwrap().get_block()).unwrap().get_fs();
        let ret = fs.cp(&mem_inode.unwrap().get_name(), &name);
        todo!(); // recursively copy children too
        Ok(ret)
    }

    pub fn mv(&self, path: &str, new_path: &str) -> bool {
        let mem_inode = self.resolve_path(path);
        if mem_inode.is_err() {
            return false;
        }

        let mut dentry = &mut self.root.get_root().lock();
        let mut parent: Option<u32> = Option::None;
        let name: String = String::new();
        for step in path {
            for child in self.forward(dentry) {
                let mut child = child.lock();
                if child.name == step {
                    parent = Option::Some(dentry.get_ino());
                    dentry = &mut child;
                    if dentry.mounted {
                        name.clear();
                    }
                    else {
                        name.push("/");
                        name.push_str(step);
                    }
                    continue;
                }
                return Err( IOError::new("Path not found".into()))
            }
        }

        mem_inode.unwrap().set_name(&name);
        mem_inode.unwrap().set_children(Vec::new());

        let mut dentry = &mut self.root.get_root().lock();
        let mut parent: Option<u32> = Option::None;
        for step in path.split("/") {
            for child in self.forward(dentry) {
                let mut child = child.lock();
                if child.name == step {
                    parent = Option::Some(dentry.get_ino());
                    dentry = &mut child;
                    break;
                }
            }
        }
        dentry.name = name.into();
        dentry.parent = parent;


        let fs = self.get_superblock(&mem_inode.unwrap().get_block()).unwrap().get_fs();
        fs.mv(&mem_inode.unwrap().get_name(), &name)
    }
}

pub fn fs_init(blocks: BlockManager, root_name: &str) -> Option<Vfs>{ 
    let root: SuperBlock = if let Option::Some(blk) = Vfs::register_filesys(blocks.by_name(root_name).unwrap()) {
        blk
    } else{
        return Option::None;
    };
    Option::Some(Vfs {
        root, 
        registered: Vec::new(),
        blocks
    })
}
