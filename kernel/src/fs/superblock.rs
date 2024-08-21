use crate::dev::block::Block;
use alloc::string::String;
use crate::sync::irq::MutexIrq;
use crate::fs::{inode::MemInode, tempfs::Tempfs, vfs::{File, Dentry}};
use alloc::collections::btree_map::BTreeMap;

// TODO: clear inode cache after a while
// TODO: change BtreeMap to store Arc<MemInode>
pub trait FileSystem {
    fn get_root_ino(&self) -> u32;
    fn read_inode(&self, inode: u32) -> Option<MemInode>;
    fn device_name(&self) -> &str;
    fn open(&self, path: &str) -> Option<File>;
    fn close(&self, file: &File) -> bool;
    fn read(&self, file: &File, buf: &mut [u8]) -> u32;
    fn write(&self, file: &File, buf: &[u8]) -> u32;
    fn create(&mut self, path: &str) -> u32;
    fn delete(&self, path: &str) -> bool;
    fn mkdir(&mut self, path: &str) -> u32;
    fn rmdir(&mut self, path: &str) -> bool;
    fn cp(&self, path: &str, name: &str) -> u32;
    fn mv(&self, path: &str, name: &str) -> bool;
}
#[derive(Clone)]
pub enum FsType{
    Tempfs(Tempfs),
}

/// This `impl` block defines a method called `unwrap` for the `FsType` enum. The `unwrap` method takes
/// a reference to `self` (an instance of `FsType`) and returns a reference to a trait object `&dyn
/// FileSystem`.
impl FsType {
    pub fn unwrap(&self) -> &dyn FileSystem {
        match self {
            FsType::Tempfs(fs) => fs,
        }
    }
}


pub struct SuperBlock{
    // name: String,
    fs: FsType,
    root: MutexIrq<Dentry>,
    inode_tree: BTreeMap<u32, MemInode>,
    dentry_tree: BTreeMap<String, Dentry>,
}


impl SuperBlock {
    pub fn new(fs: FsType) -> SuperBlock {
        SuperBlock {
            fs: fs.clone(),
            root: MutexIrq::new(
                Dentry::create_root(fs.unwrap().device_name(), 
                 fs.unwrap().get_root_ino()
                )
            ),
            inode_tree: BTreeMap::new(),
            dentry_tree: BTreeMap::new(),
        }
    }
    pub fn get_root(&self) -> &MutexIrq<Dentry> {
        &self.root
    }

    pub fn get_fs(&self) -> &dyn FileSystem {
        self.fs.unwrap()
    }

    pub fn fs_name(&self) -> &str {
        self.fs.unwrap().device_name()
    }

    pub fn lookup_inode(&self, ino: u32) -> Option<MemInode> {
        if let inode = self.inode_tree.get(&ino.clone()) {
            return inode.clone().cloned();
        }
        let inode = self.fs.unwrap().read_inode(ino);
        if inode.is_none() {
            return Option::None;
        }
        self.inode_tree.insert(ino, inode.clone().unwrap());
        Option::Some(inode.unwrap())
    }
}
