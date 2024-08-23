#![allow(unused_variables)]
#![allow(dead_code)]

use super::inode::{InodeNum, MemInode, Stat};
use crate::fs::tempfs::*;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::{format, string::String, vec, vec::Vec};

pub trait FileSystem {
    fn blkid(&self) -> Blkid;
    fn read_ino(&self, ino: InodeNum) -> MemInode;
    fn root_ino(&self) -> InodeNum;
    fn stat(&self, path: String) -> Option<Stat>;
    fn mkdir(&self, path: String) -> Option<InodeNum>;
    fn mv(&self, src_path: String, dest_path: String) -> Option<()>;
    // fn cp(&mut self, src_path: String, dest_path: String) -> Option<usize>; implemented in Vfs
    fn open(&self, path: String) -> Option<File>;
    fn close(&self, file: &File) -> Option<()>;
    fn read(&self, file: &File, buf: &[u8]) -> Option<()>;
    fn write(&self, file: File, buf: &[u8]) -> Option<()>;
    fn del(&self, path: String) -> Option<()>;
    fn create(&self, path: String) -> Option<InodeNum>;
}

pub struct File {
    mem_inode: MemInode,
}

#[derive(Clone)]
pub enum FsType {
    Tempfs(Tempfs),
}

impl FsType {
    pub fn unwrap(&self) -> &dyn FileSystem {
        match self {
            FsType::Tempfs(fs) => fs,
        }
    }
}

pub type Blkid = u32; // unique and immutable
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
    fn new(inode: MemInode, parent: usize, mounted: bool, idx: usize) -> Option<Dentry> {
        if inode.is_directory() {
            Option::Some(Dentry {
                inode,
                parent,
                contains_mountpoint: mounted,
                idx,
                dirty: true,
                children_idx: BTreeSet::new(),
            })
        } else {
            Option::None
        }
    }

    fn set_dirty(&mut self, b: bool) {
        self.dirty = b;
    }

    fn clean(&self) -> bool {
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

    fn inode(&self) -> &MemInode {
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
    path: Vec<String>,
}

impl From<String> for Path {
    fn from(value: String) -> Self {
        let mut path: Vec<String> = Vec::new();
        for s in value.split('/') {
            if !s.is_empty() {
                path.push(s.into())
            }
        }
        Path { path }
    }
}

impl Path {
    fn iter_to_parent(&self) -> impl Iterator<Item = &String> {
        let len = self.path.len();
        self.path[1..len - 1].iter()
    }

    fn parent(&self) -> Path {
        Path {
            path: self.iter_to_parent().map(|s| s.into()).collect(),
        }
    }

    fn iter(&self) -> impl Iterator<Item = &String> {
        return self.path.iter();
    }

    pub fn as_string(&self) -> String {
        self.path.join("/")
    }
}

impl Clone for Path {
    fn clone(&self) -> Self {
        Path {
            path: self.path.clone(),
        }
    }
}

pub struct Vfs {
    registered: BTreeMap<Blkid, SuperBlock>,
    root_dentry_idx: usize,
    dentries: BTreeMap<usize, Dentry>,
    next_idx: usize,
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
        self.dentries.insert(
            self.next_idx,
            Dentry::new(inode, parent, mounted, self.next_idx).unwrap(),
        );
        self.next_idx += 1;
        self.next_idx - 1
    }

    fn name_by_idx(&self, idx: usize) -> Option<&str> {
        self.dentries.get(&idx).map(|x| x.name())
    }

    fn forward(&mut self, idx: usize) -> Vec<usize> {
        let mut dentry = self.dentries.remove(&idx).unwrap();
        let children: Vec<usize> = dentry.iter_children().copied().collect();
        if dentry.clean() {
            return children;
        }
        //otherwise, read children from disk and then iterate downwards
        let block = self.registered.get(&dentry.blkid()).unwrap();
        let children_ino: Vec<MemInode> = dentry
            .inode()
            .get_disk_children()
            .unwrap()
            .map(|x: &InodeNum| -> MemInode { block.read_inode(*x) })
            .collect();

        for c in children_ino {
            let cidx = self.register_dentry(c, idx, false);
            dentry.push_child(cidx);
        }
        self.dentries.insert(idx, dentry);
        self.dentries
            .get(&idx)
            .unwrap()
            .iter_children()
            .copied()
            .collect()
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
                    next = c;
                }
            }
            if next == prev {
                return 0;
            }
            prev = next;
        }
        prev
    }

    fn children_inode_numbers(&self, idx: usize) -> Vec<InodeNum> {
        self.dentries
            .get(&idx)
            .unwrap()
            .inode()
            .get_disk_children()
            .unwrap()
            .copied()
            .collect()
    }

    fn get_subpath(&mut self, path: Path) -> (Path, u32) {
        // returns the path relative to the last mountpoint, and the filesystem associated with that mountpoint
        let mut subpath = Vec::new();
        let mut curr = 0;
        let mut blkid = self.registered.get(&0).unwrap().blkid();

        for dir in path.iter() {
            subpath.push(dir.clone());
            subpath.push("/".into());
            for child in self.forward(curr) {
                if self.name_by_idx(child).unwrap() == dir {
                    curr = child;
                    if self.dentries.get(&curr).unwrap().is_mountpoint() {
                        subpath = vec!["/".into()];
                        blkid = self
                            .registered
                            .get(&self.dentries.get(&curr).unwrap().blkid())
                            .unwrap()
                            .blkid();
                    }
                }
            }
        }

        (Path { path: subpath }, blkid)
    }
}

impl Vfs {
    pub fn stat(&mut self, path: String) -> Option<Stat> {
        let path = Path::from(path);
        let (subpath, _) = self.get_subpath(path.clone());
        let idx = self.resolve_path(path.clone());
        if idx == 0 {
            return Option::None;
        }
        let dentry = self.dentries.get_mut(&idx).unwrap();
        let fs = self.registered.get(&dentry.blkid()).unwrap();
        fs.fs.unwrap().stat(subpath.as_string())
    }

    pub fn mkdir(&mut self, path: String) -> Option<InodeNum> {
        let path = Path::from(path);
        let parent = self.resolve_path(path.parent());

        let (subpath, fs) = self.get_subpath(path);

        let fs = self.registered.get(&fs).unwrap();
        let idx = fs.fs.unwrap().mkdir(subpath.as_string());
        idx?;
        let mem_inode = fs.read_inode(idx.unwrap());

        self.register_dentry(mem_inode, parent, false);
        idx
    }

    pub fn mv(&mut self, src_path: String, dest_path: String) -> Option<()> {
        let src = Path::from(src_path.clone());
        let dest = Path::from(dest_path.clone());

        let idx = self.resolve_path(src.clone());
        if idx == 0 {
            return Option::None;
        }

        let name = dest.path.last().unwrap().clone();
        let (src_subpath, src_fs) = self.get_subpath(src.clone());
        let (dest_subpath, dest_fs) = self.get_subpath(dest.clone());
        if src_fs == dest_fs {
            let src_fs = self.registered.get(&src_fs).unwrap();
            src_fs
                .fs
                .unwrap()
                .mv(src_subpath.as_string(), dest_subpath.as_string());
            // TODO: add children here
        } else {
            self.cp(src_path.clone(), dest_path)?;
            self.del(src_path.clone())?;
        }

        let dentry = self.dentries.get_mut(&idx).unwrap();
        dentry.inode.set_name(name);
        let src_parent_idx = dentry.parent_idx();
        let src_parent = self.dentries.get_mut(&src_parent_idx).unwrap();
        src_parent.pop_child(idx);

        let dest_parent = dest.parent();
        let dest_parent_idx = self.resolve_path(dest_parent);
        if dest_parent_idx == 0 {
            return Option::None;
        }
        let dest_parent_dentry = self.dentries.get_mut(&dest_parent_idx).unwrap();
        dest_parent_dentry.push_child(idx);

        let dentry = self.dentries.get_mut(&idx).unwrap();
        dentry.parent = dest_parent_idx;

        Option::Some(())
    }

    pub fn cp(&mut self, src_path: String, dest_path: String) -> Option<usize> {
        let src = Path::from(src_path.clone());
        let dest = Path::from(dest_path.clone());

        let idx = self.resolve_path(src);
        if idx == 0 {
            return Option::None;
        }

        let new_dentry;
        let name = dest.path.last().unwrap().clone();
        let src_dentry = self.dentries.get_mut(&idx).unwrap();
        let is_directory = src_dentry.inode.is_directory();
        let idx = src_dentry.idx();

        if is_directory {
            let mut children: Vec<usize> = Vec::new();
            for c in self.forward(idx) {
                let mut src_child_path = src_path.clone();
                src_child_path.push_str(&format!("/{}", self.name_by_idx(c).unwrap()));
                let mut dest_child_path = dest_path.clone();
                dest_child_path.push_str(&format!("/{}", self.name_by_idx(c).unwrap()));
                let child_dentry = self.cp(src_child_path, dest_child_path);

                if child_dentry.is_none() {
                    continue;
                }

                children.push(child_dentry.unwrap());
            }

            let idx = self.mkdir(dest_path.clone());
            idx?;

            let dest_parent = dest.parent();
            let dest_parent_idx = self.resolve_path(dest_parent);
            if dest_parent_idx == 0 {
                return Option::None;
            }

            let (_, fs) = self.get_subpath(dest);
            let fs = self.registered.get(&fs).unwrap();
            let inode = fs.read_inode(idx.unwrap()).ino();
            let mem_inode = fs.read_inode(inode);
            new_dentry = self.register_dentry(mem_inode, dest_parent_idx, false);
            let dentry = self.dentries.get_mut(&new_dentry).unwrap();

            for c in children {
                dentry.push_child(c);
            }
        } else {
            let idx = self.create(dest_path.clone());

            let src_file = self.open(src_path);
            src_file.as_ref()?;
            let src_file = src_file.unwrap();
            let buf = Vec::new();
            self.read(&src_file, &buf);
            self.close(src_file);

            let dest_file = self.open(dest_path.clone());
            dest_file.as_ref()?;
            let dest_file = dest_file.unwrap();
            self.write(&dest_file, &buf);
            self.close(dest_file);

            let dest_parent = dest.parent();
            let dest_parent_idx = self.resolve_path(dest_parent);
            if dest_parent_idx == 0 {
                return Option::None;
            }

            let (_, fs) = self.get_subpath(dest.clone());
            let fs = self.registered.get(&fs).unwrap();
            fs.fs.unwrap().create(dest_path.clone())?;

            let mem_inode = fs.read_inode(idx.unwrap());
            new_dentry = self.register_dentry(mem_inode, dest_parent_idx, false);
        }

        Option::Some(new_dentry)
    }

    pub fn open(&mut self, path: String) -> Option<File> {
        let path = Path::from(path);
        let (subpath, fs) = self.get_subpath(path);
        let fs = self.registered.get(&fs).unwrap();
        let file = fs.fs.unwrap().open(subpath.as_string());
        file.as_ref()?;
        file
    }

    pub fn close(&self, file: File) -> Option<()> {
        let fs = self.registered.get(&file.mem_inode.blkid()).unwrap();
        fs.fs.unwrap().close(&file)?;
        Option::Some(())
    }

    pub fn read(&self, file: &File, buf: &[u8]) -> Option<()> {
        let fs = self.registered.get(&file.mem_inode.blkid()).unwrap();
        fs.fs.unwrap().read(file, buf)?;
        Option::Some(())
    }

    pub fn write(&self, file: &File, buf: &[u8]) -> Option<()> {
        let fs = self.registered.get(&file.mem_inode.blkid()).unwrap();
        fs.fs.unwrap().read(file, buf)?;
        Option::Some(())
    }

    pub fn del(&mut self, path: String) -> Option<()> {
        let pathname = Path::from(path.clone());
        let idx = self.resolve_path(pathname.clone());
        if idx == 0 {
            return Option::None;
        }

        let dentry = self.dentries.get(&idx).unwrap();
        if dentry.is_mountpoint() {
            return Option::None;
        }

        let (subpath, fs) = self.get_subpath(pathname);
        let fs = self.registered.get(&fs).unwrap();
        fs.fs.unwrap().del(subpath.as_string())?;

        let parent_idx = self.dentries.get(&idx).unwrap().parent_idx();
        let parent = self.dentries.get_mut(&parent_idx).unwrap();
        parent.pop_child(idx);
        self.dentries.remove(&idx);
        for c in self.forward(idx) {
            let mut path_to_c = path.clone();
            path_to_c.push_str(&format!("/{}", self.name_by_idx(c).unwrap()));
            self.del(path_to_c)?;
        }

        Option::Some(())
    }

    pub fn create(&mut self, path: String) -> Option<InodeNum> {
        let path = Path::from(path);
        let parent = self.resolve_path(path.parent());
        if parent == 0 {
            return Option::None;
        }

        let (subpath, fs) = self.get_subpath(path);

        let fs = self.registered.get(&fs).unwrap();
        let idx = fs.fs.unwrap().create(subpath.as_string());
        idx?;
        let mem_inode = fs.read_inode(idx.unwrap());

        self.register_dentry(mem_inode, parent, false);
        idx
    }
}
