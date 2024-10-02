use crate::fs::{FileDescriptor, ProcessFileDescriptor};
use crate::threading::thread_control_block::Pid;
use crate::vfs::{Error, FileHandle, FileSystem, INodeNum, OwnedPath, Path, Result, DirEntries, INodeType};
use alloc::{boxed::Box,
collections::{BTreeMap, btree_map::{Entry as BTreeMapEntry}},
vec};
use core::num::NonZeroUsize;

/// Maximum number of simultaneously open files for a process.
///
/// 1024 is the default on Linux.
pub const MAX_OPEN_FILES: u16 = 1024;
/// Maximum number of simultaneous mounts.
pub const MAX_MOUNT_POINTS: u16 = 256;

/// Manages a single file system
struct FileSystemManager<F: FileSystem> {
    fs: F,
    open_file_count: BTreeMap<INodeNum, NonZeroUsize>,
    open_files: BTreeMap<ProcessFileDescriptor, F::FileHandle>,
    directories: BTreeMap<INodeNum, DirEntries>,
}

impl<F: FileSystem> FileSystemManager<F> {
    fn new(fs: F) -> Self {
        Self {
            fs,
            open_file_count: BTreeMap::new(),
            open_files: BTreeMap::new(),
            directories: BTreeMap::new(),
        }
    }
    fn directory_entries(&mut self, inode: INodeNum) -> Result<&mut DirEntries> {
        if !self.directories.contains_key(&inode) {
            // TODO: consider converting this to a BTreeMap.
            let mut dir_fh = self.fs.open(inode)?;
            let entries = self.fs.readdir(&mut dir_fh);
            if !self.open_file_count.contains_key(&inode) {
                self.fs.release(inode);
            }
            let entries = entries?;
            self.directories.insert(inode, entries);
        }
        Ok(self.directories.get_mut(&inode).unwrap())
    }
    /// get inode number corresponding to path
    fn inode_for_path(&mut self, path: &Path) -> Result<INodeNum> {
        let root_inode = self.fs.root();
        if path == "/" {
            return Ok(root_inode);
        }
        assert!(path.len() > 1, "path is not absolute");
        let mut inode = root_inode;
        let mut i = 1;
        let mut parent_inodes = vec![];
        loop {
            let (component, is_last) = match path[i..].find('/') {
                Some(l) => (&path[i..i + l], false),
                None => (&path[i..], true),
            };
            if component == ".." {
                inode = parent_inodes.pop().unwrap_or(root_inode);
            }
            if component == "." {
                continue;
            }
            let dir_entries = self.directory_entries(inode)?;
            let mut found = false;
            for entry in &*dir_entries {
                if entry.name == component {
                    match entry.r#type {
                        INodeType::Link => todo!("symlink handling"),
                        INodeType::File => {
                            if !is_last {
                                return Err(Error::NotDirectory)
                            }
                        }
                        INodeType::Directory => {
                            parent_inodes.push(inode);
                        }
                    }
                    inode = entry.inode;
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(Error::NotFound);
            }
            if is_last {
                break;
            }
            i += component.len() + 1;
        }
        Ok(inode)
    }
}

/// Unfortunately `FileSystemManager<dyn FileSystem>` doesn't work (we'd have to specify the
/// FileHandle type). So we need a new trait to be able to create dynamic objects
/// which can use different file systems.
trait FileSystemManagerTrait {
    fn open(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()>;
    fn create(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()>;
    fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()>;
    fn sync(&mut self) -> Result<()>;
    fn can_be_safely_unmounted(&self) -> bool;
}

struct TempOpen<'a, F: FileSystem> {
    fs: &'a mut F,
    already_open: bool,
    handle: F::FileHandle,
}

impl<F: FileSystem> Drop for TempOpen<'_, F> {
    fn drop(&mut self) {
        if !self.already_open {
            self.fs.release(self.handle.inode());
        }
    }
}

fn temp_open<F: FileSystem>(fs: &mut F, inode: INodeNum, already_open: bool) -> Result<TempOpen<'_, F>> {
    let handle = fs.open(inode)?;
    Ok(TempOpen {
        fs,
        already_open,
        handle,
    })
}

impl<F: FileSystem> FileSystemManagerTrait for FileSystemManager<F> {
    fn open(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()> {
        let inode = self.inode_for_path(path)?;
        let handle = self.fs.open(inode)?;
        match self.open_file_count.entry(inode) {
            BTreeMapEntry::Occupied(mut o) => {
                let count = o.get_mut();
                *count = count.checked_add(1).expect("shouldn't overflow usize");
            }
            BTreeMapEntry::Vacant(v) => {
                v.insert(NonZeroUsize::new(1).unwrap());
            }
        }
        let _prev = self.open_files.insert(fd, handle);
        debug_assert!(_prev.is_none(), "duplicate fd");
        Ok(())
    }
    fn create(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()> {
        let Some(final_slash) = path.rfind('/') else {
            panic!("not an absolute path");
        };
        let dir = &path[..final_slash];
        let name = &path[final_slash + 1..];
        if name.is_empty() {
            // e.g. create("foo/")
            return Err(Error::IsDirectory);
        }
        let dir_inode = self.inode_for_path(dir)?;
        let mut dir = temp_open(&mut self.fs, dir_inode, self.open_file_count.contains_key(&dir_inode));
        self.fs.create(&mut dir.handle, name)?;
        Ok(())
    }
    fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()> {
        let handle = self.open_files.remove(&fd).ok_or(Error::BadFd)?;
        let inode = handle.inode();
        let ref_count = self.open_file_count.get_mut(&inode).unwrap();
        if let Some(n) = NonZeroUsize::new(ref_count.get() - 1) {
            *ref_count = n;
            return Ok(());
        }
        self.open_file_count.remove(&inode);
        self.fs.release(inode);
        Ok(())
    }
    fn can_be_safely_unmounted(&self) -> bool {
        self.open_files.is_empty()
    }
    fn sync(&mut self) -> Result<()> {
        self.fs.sync()
    }
}

type FileSystemID = u16;

#[derive(Debug)]
struct OpenFile {
    fs: FileSystemID,
}

pub struct RootFileSystem {
    file_systems: [Option<Box<dyn FileSystemManagerTrait>>; MAX_MOUNT_POINTS as usize],
    mount_points: BTreeMap<OwnedPath, FileSystemID>,
    open_files: BTreeMap<ProcessFileDescriptor, OpenFile>,
}

impl RootFileSystem {
    pub const fn new() -> Self {
        Self {
            file_systems: [const { None }; MAX_MOUNT_POINTS as usize],
            mount_points: BTreeMap::new(),
            open_files: BTreeMap::new(),
        }
    }
    fn resolve_path<'a>(&self, path: &'a Path) -> Result<(FileSystemID, &'a Path)> {
        let mut result = None;
        for i in 0..path.len() {
            if path.as_bytes()[i] == b'/' {
                if let Some(id) = self.mount_points.get(&path[..i]) {
                    result = Some((*id, &path[i..]));
                }
            }
        }
        result.ok_or(Error::NotFound)
    }
    fn new_fd(&mut self, fs: FileSystemID, pid: Pid) -> Result<ProcessFileDescriptor> {
        for fd in 0..MAX_OPEN_FILES as FileDescriptor {
            let fd = ProcessFileDescriptor { pid, fd };
            if let alloc::collections::btree_map::Entry::Vacant(entry) = self.open_files.entry(fd) {
                entry.insert(OpenFile { fs });
                return Ok(fd);
            }
        }
        Err(Error::TooManyOpenFiles)
    }
    pub fn mount<F: FileSystem + 'static>(&mut self, path: &Path, fs: F) -> Result<()> {
        // verify that path is an empty directory
        if path == "/" {
            if !self.mount_points.is_empty() {
                return Err(Error::NotEmpty);
            }
        } else {
            todo!("check that path is an empty directory")
        }
        // add FS
        let mut fs_id = None;
        for id in 0..MAX_MOUNT_POINTS as usize {
            if self.file_systems[id].is_none() {
                self.file_systems[id] = Some(Box::new(FileSystemManager::new(fs)));
                fs_id = Some(id as FileSystemID);
                break;
            }
        }
        let Some(fs_id) = fs_id else {
            // Maybe this isn't the best error to return here?
            // Seems unlikely that this would happen in any case.
            return Err(Error::NoSpace);
        };
        self.mount_points.insert(path.into(), fs_id);
        Ok(())
    }
    pub fn unmount(&mut self, path: &Path) -> Result<()> {
        let fs_id = *self.mount_points.get(path).ok_or(Error::NotFound)?;
        let fs = self.file_systems[fs_id as usize].as_mut().unwrap();
        if !fs.can_be_safely_unmounted() {
            return Err(Error::FileSystemInUse);
        }
        fs.sync()?;
        self.file_systems[fs_id as usize] = None;
        self.mount_points.remove(path);
        Ok(())
    }
    pub fn open(&mut self, path: &Path, pid: Pid) -> Result<ProcessFileDescriptor> {
        let (fs, path) = self.resolve_path(path)?;
        let fd = self.new_fd(fs, pid)?;
        let fs = self.file_systems[fs as usize].as_mut().unwrap();
        let result = fs.open(path, fd);
        if let Err(e) = result {
            self.open_files.remove(&fd);
            return Err(e)
        }
        Ok(fd)
    }
    pub fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()> {
        let fs = self.open_files.get(&fd).ok_or(Error::BadFd)?.fs;
        let fs = self.file_systems[fs as usize].as_mut().unwrap();
        fs.close(fd)?;
        self.open_files.remove(&fd);
        Ok(())
    }
}
