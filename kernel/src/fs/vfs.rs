#![allow(unused_variables)]
#![allow(dead_code)]
use crate::fs::tempfs::*;
use alloc::collections::BTreeSet;
// use crate::sync::irq::MutexIrq;
use alloc::{vec::Vec, string::String};
use super::inode::{MemInode, Stat, InodeNum};
use alloc::collections::btree_map::BTreeMap;


pub trait FileSystem {
    fn blkid(&self) -> Blkid;
    fn read_ino(&self, ino: InodeNum) -> MemInode;
    fn root_ino(&self) -> InodeNum;
}

#[derive(Clone)]
pub enum FsType{
    Tempfs(Tempfs),
}

impl FsType {
    pub fn unwrap(&self) -> &dyn FileSystem {
        match self {
            FsType::Tempfs(fs) => fs,
        }
    }
}

pub type Blkid = u32;
pub struct SuperBlock {
    root_ino: InodeNum,
    fs: FsType,
    mounted: bool,
    mountpoint: String,
}
impl SuperBlock {
    pub fn blkid(&self) -> Blkid {
        self.fs.unwrap().blkid()
    }
    
    pub fn root_ino(&self) -> InodeNum {
        self.root_ino
    }

    pub fn read_inode(&self, ino: InodeNum) -> MemInode {
        let fs: &dyn FileSystem = self.fs.unwrap();
        fs.read_ino(ino)
    }


    
}


// For internal VFS use only
struct Dentry {
    inode: MemInode,
    // Whether this directory or any of its children is a mountpoint, don't remove from vfs if so
    contains_mountpoint: bool, 
    // Dentry links for the VFS 
    parent: usize,
    idx: usize,
    // Whether we need to read the disk to load dentries for children
    dirty: bool,
    // Map of name -> child
    children_idx: BTreeSet<usize>,
}
impl Dentry {
    fn new(inode: MemInode, parent: usize, mounted: bool, idx: usize) -> Option<Dentry>{
        if inode.is_directory() {
            Option::Some(
                Dentry {
                    inode,
                    parent, 
                    contains_mountpoint: mounted,
                    idx,
                    dirty: true,
                    children_idx: BTreeSet::new()
                }
            )
        } else{
            Option::None
        }
    }

    fn set_dirty(&mut self, b:  bool){
            self.dirty =b; 
    }

    fn clean(&self) -> bool{
        !self.dirty
    }

    fn push_child(&mut self, idx: usize) {
        self.children_idx.insert(idx);
    }

    fn pop_child(&mut self, idx: usize) -> bool {
        self.children_idx.remove(&idx)
    }

    fn iter_children(&self) -> impl Iterator<Item = &usize> {
        self.children_idx.iter()
    }
    

    fn idx(&self) -> usize {
        self.idx
    }

    fn parent_idx(&self) -> usize {
        self.parent
    }

    fn inode(&self) -> &MemInode{
        &self.inode
    }

    fn name(&self) -> &str {
        self.inode.name()
    }

    fn ino(&self) -> InodeNum {
        self.inode.ino()
    }

    fn blkid(&self) -> Blkid {
        self.inode.blkid()
    }

    fn is_mountpoint(&self) -> bool {
        self.contains_mountpoint
    }

}


struct Path {
    path: Vec<String>   
}

impl From<String> for Path {
    fn from(value: String) -> Self {
        let mut path: Vec<String> = Vec::new();
        for s in value.split('/') {
            if !s.is_empty() {
                path.push(s.into())
            }
        }
    Path {
            path
        }
    }
}

impl Path {

    fn iter_to_parent(&self) -> impl Iterator<Item = &String>{
        let len = self.path.len();
        return self.path[1..len-1].iter()
    }

    fn parent(&self) -> Path {
        Path {
            path: self.iter_to_parent().map(|s| s.into()).collect() 
        }
    }

    fn iter(&self) -> impl Iterator<Item = &String> {
        return self.path.iter()
    }

}


pub struct Vfs {
    registered: BTreeMap<Blkid, SuperBlock>,
    root_dentry_idx: usize,
    dentries: BTreeMap<usize, Dentry>,
    next_idx : usize,
}

impl Vfs {
    pub fn new(root: SuperBlock, all_blocks: SuperBlock) -> Vfs {
        let mut registered: BTreeMap<Blkid, SuperBlock> = BTreeMap::new();
        let root_blkid = root.blkid();
        registered.insert(root_blkid, root);
        let mut rtn = Vfs {
            root_dentry_idx: 0,
            dentries: BTreeMap::new(),
            registered,
            next_idx: 0, 
        };
        let root = rtn.registered.get_mut(&root_blkid).unwrap();
        let root_ino = root.read_inode(root.root_ino());
        rtn.register_dentry(root_ino, 0, true);
        rtn
    }
    fn register_dentry(&mut self, inode: MemInode, parent: usize, mounted: bool) -> usize {
        self.dentries.insert(self.next_idx, Dentry::new(inode, parent, mounted, self.next_idx).unwrap());
        self.next_idx += 1;
        self.next_idx-1
    }

    fn name_by_idx(&self, idx: usize) -> Option<&str> {
        self.dentries.get(&idx).map(|x|x.name())
    }

    //TODO reevaluate mountpoints after free
    fn forward(&mut self, idx: usize) -> Vec<usize> {
        let mut dentry = self.dentries.remove(&idx).unwrap();
        let children: Vec<usize> = dentry.iter_children().copied().collect();
        if dentry.clean() {
            return children        }
        //otherwise, read children from disk and then iterate downwoards 
        let block  = self.registered.get(&dentry.blkid()).unwrap();
        let children_ino: Vec<MemInode> = dentry.inode().get_disk_children()
            .unwrap()
            .map(|x: &InodeNum| -> MemInode {block.read_inode(*x)})
            .collect();
        for c in children_ino {
            let cidx = self.register_dentry(c, idx, false);
            dentry.push_child(cidx);
        }
        self.dentries.insert(idx, dentry);
        self.dentries.get(&idx).unwrap().iter_children().copied().collect()
    }
     /* 
        Returns the dentry index associated with the absolute path  
        ex: resolve_path("/a/b/c/d") should give Dentry for d
     */
    fn resolve_path(&mut self, path: Path) -> usize {
        let mut prev = 0;
        for dir in path.iter() {
            let mut next = prev;
            for c in self.forward(prev) {
                if self.name_by_idx(c).unwrap() == dir {
                   next = c 
                }
            }
            if next == prev {
                return 0
            }
            prev = next;
        }
        prev 
    }

    fn children_inode_numbers(&self, idx: usize) -> Vec<InodeNum>{
        self.dentries.get(&idx).unwrap().inode().get_disk_children().unwrap().copied().collect() 
    }

    //TODO: Remove Dentries
}

//TODO: Yahya refactor
impl Vfs {
    pub fn stat(&mut self, path: String) -> Option<Stat> {
        todo!()
    }

    pub fn mkdir(&self, path: String) {

        todo!()
    }

    pub fn mv() {
        todo!()
    }

    pub fn cp() {
        todo!()

    }

}
