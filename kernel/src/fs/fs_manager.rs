use crate::fs::{FileDescriptor, ProcessFileDescriptor};
use crate::sync::mutex::Mutex;
use crate::threading::thread_control_block::Pid;
use crate::vfs::{
    DirEntries, Error, FileHandle, FileInfo, FileSystem, INodeNum, INodeType, OwnedPath, Path,
    Result,
};
use alloc::{
    boxed::Box,
    collections::{btree_map::Entry as BTreeMapEntry, BTreeMap},
    format,
    string::String,
    vec,
    vec::Vec,
};
use core::num::NonZeroUsize;

/// Possible places to seek from
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SeekFrom {
    Start,
    Current,
    End,
}

/// Mode for opening a file
#[derive(Debug, Copy, Clone)]
pub enum Mode {
    /// Open existing file for read/write access
    ReadWrite,
    /// Open or create file for read/write access
    CreateReadWrite,
    // could add ReadOnly, WriteOnly, etc. here
    // - depends whether we want support for file permissions
    // (if not, we could just do that at the libc level)
}

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

struct TempOpen<F: FileSystem> {
    handle: F::FileHandle,
}

impl<F: FileSystem> Drop for TempOpen<F> {
    fn drop(&mut self) {
        panic!("temporarily-open file dropped — make sure you call FileSystemManager::temp_close instead!")
    }
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

    fn temp_open(&mut self, inode: INodeNum) -> Result<TempOpen<F>> {
        let handle = self.fs.open(inode)?;
        Ok(TempOpen { handle })
    }
    fn temp_open_path(&mut self, path: &Path) -> Result<TempOpen<F>> {
        let inode = self.inode_for_path(path)?;
        self.temp_open(inode)
    }
    fn temp_close(&mut self, file: TempOpen<F>) {
        let inode = file.handle.inode();
        if self.open_file_count.contains_key(&inode) {
            self.fs.release(inode);
        }
        core::mem::forget(file);
    }

    fn directory_entries(&mut self, inode: INodeNum) -> Result<&mut DirEntries> {
        #[allow(clippy::map_entry)] // can't use entry() here because we're borrowing self mutably
        if !self.directories.contains_key(&inode) {
            let mut dir = self.temp_open(inode)?;
            // TODO: consider converting entries to a BTreeMap.
            let entries = self.fs.readdir(&mut dir.handle);
            self.temp_close(dir);
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
                                return Err(Error::NotDirectory);
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
    fn open_file_handle(&mut self, fd: ProcessFileDescriptor, handle: F::FileHandle) -> Result<()> {
        match self.open_file_count.entry(handle.inode()) {
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
}

/// Unfortunately `FileSystemManager<dyn FileSystem>` doesn't work (we'd have to specify the
/// FileHandle type). So we need a new trait to be able to create dynamic objects
/// which can use different file systems.
trait FileSystemManagerTrait: Send + Sync {
    fn open(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()>;
    fn create(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()>;
    fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()>;
    fn read(&mut self, fd: ProcessFileDescriptor, offset: u64, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, fd: ProcessFileDescriptor, offset: u64, buf: &[u8]) -> Result<usize>;
    fn sync(&mut self) -> Result<()>;
    fn exists(&mut self, path: &Path) -> Result<bool>;
    fn mkdir(&mut self, path: &Path) -> Result<()>;
    fn can_be_safely_unmounted(&self) -> bool;
    fn stat(&mut self, fd: ProcessFileDescriptor) -> Result<FileInfo>;
    fn size_of_file(&mut self, fd: ProcessFileDescriptor) -> Result<u64>;
}

/// get parent directory and name of absolute path
/// e.g. /foo/bar => "/foo", "bar"
fn dirname_and_filename(path: &Path) -> (&Path, &Path) {
    let Some(final_slash) = path.rfind('/') else {
        panic!("not an absolute path");
    };
    let dir = if final_slash == 0 {
        "/"
    } else {
        &path[..final_slash]
    };
    let name = &path[final_slash + 1..];
    (dir, name)
}

impl<F: FileSystem> FileSystemManagerTrait for FileSystemManager<F> {
    fn open(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()> {
        let inode = self.inode_for_path(path)?;
        let handle = self.fs.open(inode)?;
        self.open_file_handle(fd, handle)
    }
    fn create(&mut self, path: &Path, fd: ProcessFileDescriptor) -> Result<()> {
        let (dir, name) = dirname_and_filename(path);
        if name.is_empty() {
            // e.g. create("foo/")
            return Err(Error::IsDirectory);
        }
        let dir_inode = self.inode_for_path(dir)?;
        let mut dir = self.temp_open(dir_inode)?;
        let file = self.fs.create(&mut dir.handle, name);
        self.temp_close(dir);
        let file = file?;
        // add file to directory entry cache
        self.directory_entries(dir_inode)?
            .add(file.inode(), INodeType::File, name);
        self.open_file_handle(fd, file)
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
    fn mkdir(&mut self, path: &Path) -> Result<()> {
        let (parent_dir, name) = dirname_and_filename(path);
        if name.is_empty() {
            // e.g. turn mkdir("/foo/") into mkdir("/foo")
            return self.mkdir(parent_dir);
        }
        let mut parent_dir = self.temp_open_path(parent_dir)?;
        let result = self.fs.mkdir(&mut parent_dir.handle, name);
        self.temp_close(parent_dir);
        result
    }
    fn exists(&mut self, path: &Path) -> Result<bool> {
        match self.inode_for_path(path) {
            Err(Error::NotFound) => Ok(false),
            Err(e) => Err(e),
            Ok(_) => Ok(true),
        }
    }
    fn read(&mut self, fd: ProcessFileDescriptor, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let handle = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        self.fs.read(handle, offset, buf)
    }
    fn write(&mut self, fd: ProcessFileDescriptor, offset: u64, buf: &[u8]) -> Result<usize> {
        let handle = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        self.fs.write(handle, offset, buf)
    }
    fn stat(&mut self, fd: ProcessFileDescriptor) -> Result<FileInfo> {
        let handle = self.open_files.get(&fd).ok_or(Error::BadFd)?;
        self.fs.stat(handle)
    }
    fn size_of_file(&mut self, fd: ProcessFileDescriptor) -> Result<u64> {
        Ok(self.stat(fd)?.size)
    }
}

type FileSystemID = u16;

#[derive(Debug)]
enum OpenFile {
    /// regular file
    Regular { fs: FileSystemID, offset: u64 },
    /// standard output
    StdOut,
    /// /dev/null (discards reads/writes)
    Null,
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
    /// Determine which filesystem a path belongs to, and return the path relative to the filesystem.
    ///
    /// This can only fail if / isn't mounted.
    fn resolve_path<'a>(&self, path: &'a Path) -> Result<(FileSystemID, &'a Path)> {
        let mut result = None;
        for i in 0..path.len() {
            if path.as_bytes()[i] == b'/' {
                let dir = if i == 0 { "/" } else { &path[..i] };
                if let Some(id) = self.mount_points.get(dir) {
                    result = Some((*id, &path[i..]));
                }
            }
        }
        result.ok_or(Error::NotFound)
    }
    fn new_fd(&mut self, pid: Pid, file_info: OpenFile) -> Result<ProcessFileDescriptor> {
        for fd in 0..MAX_OPEN_FILES as FileDescriptor {
            let fd = ProcessFileDescriptor { pid, fd };
            if let alloc::collections::btree_map::Entry::Vacant(entry) = self.open_files.entry(fd) {
                entry.insert(file_info);
                return Ok(fd);
            }
        }
        Err(Error::TooManyOpenFiles)
    }
    pub fn mount<F: FileSystem + 'static>(&mut self, path: &Path, fs: F) -> Result<()> {
        // verify that path doesn't already exist
        if path == "/" {
            if !self.mount_points.is_empty() {
                return Err(Error::Exists);
            }
        } else {
            let (fs, name) = self.resolve_path(path)?;
            let fs = self.file_systems[fs as usize].as_mut().unwrap();
            let (parent, _) = dirname_and_filename(name);
            if !fs.exists(parent)? {
                // e.g. mount /foo/bar when /foo doesn't exist
                return Err(Error::NotFound);
            }
            if fs.exists(name)? {
                return Err(Error::Exists);
            }
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
        for mount_point in self.mount_points.keys() {
            if path != mount_point && mount_point.starts_with(path) {
                // e.g. can't unmount /foo while /foo/bar is mounted
                return Err(Error::FileSystemInUse);
            }
        }
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
    pub fn open(&mut self, path: &Path, pid: Pid, mode: Mode) -> Result<FileDescriptor> {
        let (fs, path) = self.resolve_path(path)?;
        let fd = self.new_fd(pid, OpenFile::Regular { fs, offset: 0 })?;
        let fs = self.file_systems[fs as usize].as_mut().unwrap();
        let result = match mode {
            Mode::ReadWrite => fs.open(path, fd),
            Mode::CreateReadWrite => fs.create(path, fd),
        };
        if let Err(e) = result {
            self.open_files.remove(&fd);
            return Err(e);
        }
        Ok(fd.fd)
    }
    pub fn open_stdout(&mut self, pid: Pid) -> Result<FileDescriptor> {
        let fd = self.new_fd(pid, OpenFile::StdOut)?;
        Ok(fd.fd)
    }
    pub fn open_null(&mut self, pid: Pid) -> Result<FileDescriptor> {
        let fd = self.new_fd(pid, OpenFile::Null)?;
        Ok(fd.fd)
    }
    /// Close an open file
    ///
    /// If this returns an error other than [`Error::BadFd`], the file is still closed,
    /// and you should not try to close it again (as on Linux).
    pub fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()> {
        let mut result = Ok(());
        let file_info = self.open_files.get(&fd).ok_or(Error::BadFd)?;
        if let OpenFile::Regular { fs, .. } = file_info {
            let fs = self.file_systems[*fs as usize].as_mut().unwrap();
            result = fs.close(fd);
        }
        // don't need to do anything for non-regular files
        self.open_files.remove(&fd);
        result
    }
    pub fn mkdir(&mut self, path: &Path) -> Result<()> {
        let (fs, path) = self.resolve_path(path)?;
        let fs = self.file_systems[fs as usize].as_mut().unwrap();
        fs.mkdir(path)
    }
    pub fn read(&mut self, fd: ProcessFileDescriptor, buf: &mut [u8]) -> Result<usize> {
        let file_info = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        match file_info {
            OpenFile::Regular { fs, offset } => {
                let fs = self.file_systems[*fs as usize].as_mut().unwrap();
                let read_count = fs.read(fd, *offset, buf)?;
                *offset += read_count as u64;
                Ok(read_count)
            }
            OpenFile::StdOut => {
                // shouldn't read from stdout
                Err(Error::BadFd)
            }
            OpenFile::Null => Ok(0),
        }
    }
    pub fn write(&mut self, fd: ProcessFileDescriptor, buf: &[u8]) -> Result<usize> {
        let file_info = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        match file_info {
            OpenFile::Regular { fs, offset } => {
                let fs = self.file_systems[*fs as usize].as_mut().unwrap();
                let write_count = fs.write(fd, *offset, buf)?;
                *offset += write_count as u64;
                Ok(write_count)
            }
            OpenFile::StdOut => {
                use core::fmt::Write;
                let string = String::from_utf8_lossy(buf);
                // SAFETY: no other mut references to VIDEO_MEMORY_WRITER here
                let result = unsafe {
                    kidneyos_shared::video_memory::VIDEO_MEMORY_WRITER.write_str(&string)
                };
                if let Err(e) = result {
                    Err(Error::IO(format!("{e}")))
                } else {
                    Ok(buf.len())
                }
            }
            OpenFile::Null => Ok(buf.len()),
        }
    }
    pub fn lseek(
        &mut self,
        fd: ProcessFileDescriptor,
        whence: SeekFrom,
        offset: i64,
    ) -> Result<i64> {
        let file_info = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        if let OpenFile::Regular {
            fs,
            offset: file_offset,
        } = file_info
        {
            let new_offset = offset
                .checked_add(match whence {
                    SeekFrom::Start => 0,
                    SeekFrom::Current => *file_offset as i64,
                    SeekFrom::End => {
                        let fs = self.file_systems[*fs as usize].as_mut().unwrap();
                        fs.size_of_file(fd)? as i64
                    }
                })
                .ok_or(Error::BadOffset)?;
            *file_offset = u64::try_from(new_offset).map_err(|_| Error::BadOffset)?;
            Ok(new_offset)
        } else {
            Err(Error::IllegalSeek)
        }
    }
    /// Open the standard input, output, error files for pid.
    ///
    /// Panics if the file descriptors 0, 1, 2 are already in use for pid.
    pub fn open_standard_fds(&mut self, pid: Pid) {
        // for now, ignore stdin (we don't have keyboard input set up yet)
        let stdin = self.open_null(pid).unwrap();
        assert_eq!(stdin, 0);
        let stdout = self.open_stdout(pid).unwrap();
        assert_eq!(stdout, 1);
        // stderr and stdout can just go to the same place for now
        let stderr = self.open_stdout(pid).unwrap();
        assert_eq!(stderr, 2);
    }
    /// Close all open files belonging to process
    ///
    /// This should be called when the process exits/is killed.
    /// All errors that occur while closing files are ignored.
    pub fn close_all(&mut self, pid: Pid) {
        let fds: Vec<FileDescriptor> = self
            .open_files
            .keys()
            .filter_map(|fd| if fd.pid == pid { Some(fd.fd) } else { None })
            .collect();
        for fd in fds {
            let _ = self.close(ProcessFileDescriptor { pid, fd });
        }
    }
}

pub static ROOT: Mutex<RootFileSystem> = Mutex::new(RootFileSystem::new());

#[cfg(test)]
mod test {
    use super::*;
    use crate::vfs::tempfs::TempFS;
    // open file for fake PID of 1 for testing
    fn open(root: &mut RootFileSystem, path: &Path, mode: Mode) -> Result<ProcessFileDescriptor> {
        let pid = 1;
        let fd = root.open(path, pid, mode)?;
        Ok(ProcessFileDescriptor { fd, pid })
    }
    #[test]
    fn test_one_filesystem_simple() {
        let mut root = RootFileSystem::new();
        let fs = TempFS::new();
        root.mount("/", fs).unwrap();
        let file = open(&mut root, "/foo", Mode::CreateReadWrite).unwrap();
        assert_eq!(root.write(file, b"test data").unwrap(), 9);
        root.close(file).unwrap();
        let file = open(&mut root, "/foo", Mode::ReadWrite).unwrap();
        let mut buf = [0; 10];
        assert_eq!(root.read(file, &mut buf).unwrap(), 9);
        assert_eq!(&buf, b"test data\0");
        root.close(file).unwrap();
        root.unmount("/").unwrap();
    }
    #[test]
    fn test_multiple_filesystems_simple() {
        let mut root = RootFileSystem::new();
        let fs = TempFS::new();
        root.mount("/", fs).unwrap();
        let fs2 = TempFS::new();
        root.mount("/2", fs2).unwrap();
        let fs3 = TempFS::new();
        root.mount("/2/3", fs3).unwrap();
        for path in ["/foo", "/2/foo", "/2/3/foo"] {
            let file = open(&mut root, path, Mode::CreateReadWrite).unwrap();
            // we shouldn't be allowed to unmount the FS file is contained in while it's open
            assert!(matches!(
                root.unmount(dirname_and_filename(path).0),
                Err(Error::FileSystemInUse)
            ));
            assert_eq!(root.write(file, b"test data").unwrap(), 9);
            root.close(file).unwrap();
            let file = open(&mut root, path, Mode::ReadWrite).unwrap();
            let mut buf = [0; 10];
            assert_eq!(root.read(file, &mut buf).unwrap(), 9);
            assert_eq!(&buf, b"test data\0");
            root.close(file).unwrap();
        }
        assert!(matches!(root.unmount("/2"), Err(Error::FileSystemInUse)));
        root.unmount("/2/3").unwrap();
        root.unmount("/2").unwrap();
        root.unmount("/").unwrap();
    }
}
