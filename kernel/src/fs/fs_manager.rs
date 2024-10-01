use crate::fs::{FileDescriptor, ProcessFileDescriptor};
use crate::threading::thread_control_block::Pid;
use crate::vfs::{Error, FileHandle, FileSystem, INodeNum, OwnedPath, Path, Result};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;

/// Maximum number of simultaneously open files for a process.
///
/// 1024 is the default on Linux.
pub const MAX_OPEN_FILES: u16 = 1024;
/// Maximum number of simultaneous mounts.
pub const MAX_MOUNT_POINTS: u16 = 256;

/// Manages a single file system
struct FileSystemManager<F: FileSystem> {
    fs: F,
    open_file_count: BTreeMap<INodeNum, u32>,
    open_files: BTreeMap<ProcessFileDescriptor, F::FileHandle>,
}

impl<F: FileSystem> FileSystemManager<F> {
    fn new(fs: F) -> Self {
        Self {
            fs,
            open_file_count: BTreeMap::new(),
            open_files: BTreeMap::new(),
        }
    }
}

/// Unfortunately `FileSystemManager<dyn FileSystem>` doesn't work (we'd have to specify the
/// FileHandle type). So we need a new trait to be able to create dynamic objects
/// which can use different file systems.
trait FileSystemManagerTrait {
    fn open(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()>;
    fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()>;
    fn sync(&mut self) -> Result<()>;
    fn can_be_safely_unmounted(&self) -> bool;
}

impl<F: FileSystem> FileSystemManagerTrait for FileSystemManager<F> {
    fn open(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()> {
        let _ = path;
        let _ = fd;
        todo!("lookup directory containing path, and open the right file in it.")
    }
    fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()> {
        let handle = self.open_files.remove(&fd).ok_or(Error::BadFd)?;
        let inode = handle.inode();
        let ref_count = self.open_file_count.get_mut(&inode).unwrap();
        if *ref_count > 1 {
            *ref_count -= 1;
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

impl Default for RootFileSystem {
    fn default() -> Self {
        Self {
            file_systems: [(); MAX_MOUNT_POINTS as usize].map(|()| None),
            mount_points: BTreeMap::new(),
            open_files: BTreeMap::new(),
        }
    }
}

impl RootFileSystem {
    pub fn new() -> Self {
        Self::default()
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
        fs.open(path, fd)?;
        Ok(fd)
    }
    pub fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()> {
        let fs = self.open_files[&fd].fs;
        let fs = self.file_systems[fs as usize].as_mut().unwrap();
        fs.close(fd)
    }
}
