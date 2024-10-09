use crate::fs::{FileDescriptor, ProcessFileDescriptor};
use crate::sync::mutex::Mutex;
use crate::threading::thread_control_block::Pid;
use crate::threading::thread_control_block::ProcessControlBlock;
use crate::vfs::{
    DirEntries, Error, FileHandle, FileInfo, FileSystem, INodeNum, INodeType, Path, Result,
};
use alloc::borrow::Cow;
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
/// Maximum number of nested symbolic links
pub const MAX_LEVEL_OF_LINKS: usize = 32;

struct Directory {
    entries: Option<DirEntries>,
    parent: INodeNum,
    mount: Option<FileSystemID>,
}

impl Directory {
    fn add(&mut self, inode: INodeNum, r#type: INodeType, name: &Path) {
        self.entries
            .as_mut()
            .expect("Directory::add called before directory entries were scanned")
            .add(inode, r#type, name)
    }
    fn is_empty(&self) -> bool {
        self.entries
            .as_ref()
            .expect("Directory::is_empty called before directory entries were scanned")
            .entries
            .is_empty()
    }
}

/// Manages a single file system
struct FileSystemManager<F: FileSystem> {
    fs: F,
    mount_point: Option<(FileSystemID, INodeNum)>,
    open_file_count: BTreeMap<INodeNum, NonZeroUsize>,
    open_files: BTreeMap<ProcessFileDescriptor, F::FileHandle>,
    directories: BTreeMap<INodeNum, Directory>,
    mount_count: u32,
}

struct TempOpen<F: FileSystem> {
    handle: F::FileHandle,
}

impl<F: FileSystem> Drop for TempOpen<F> {
    fn drop(&mut self) {
        panic!("temporarily-open file dropped — make sure you call FileSystemManager::temp_close instead!")
    }
}

fn temp_open<F: FileSystem>(fs: &mut F, inode: INodeNum) -> Result<TempOpen<F>> {
    let handle = fs.open(inode)?;
    Ok(TempOpen { handle })
}

fn temp_close<F: FileSystem>(
    fs: &mut F,
    file: TempOpen<F>,
    open_file_count: &BTreeMap<INodeNum, NonZeroUsize>,
) {
    let inode = file.handle.inode();
    if open_file_count.contains_key(&inode) {
        fs.release(inode);
    }
    // prevent drop from running
    core::mem::forget(file);
}

impl<F: FileSystem> FileSystemManager<F> {
    fn new(fs: F, mount_point: Option<(FileSystemID, INodeNum)>) -> Self {
        let root_ino = fs.root();
        let mut me = Self {
            fs,
            open_file_count: BTreeMap::new(),
            open_files: BTreeMap::new(),
            directories: BTreeMap::new(),
            mount_point,
            mount_count: 0,
        };
        me.directories.insert(
            root_ino,
            Directory {
                entries: None,
                mount: None,
                parent: root_ino,
            },
        );
        // ensure root directory entries are in cache
        let _ = me.lookup(root_ino, "x");
        me
    }

    fn temp_open(&mut self, inode: INodeNum) -> Result<TempOpen<F>> {
        temp_open(&mut self.fs, inode)
    }
    fn temp_close(&mut self, file: TempOpen<F>) {
        temp_close(&mut self.fs, file, &self.open_file_count)
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
    fn root(&self) -> INodeNum;
    fn mount_point(&self) -> Option<(FileSystemID, INodeNum)>;
    fn lookup(&mut self, dir: INodeNum, entry: &Path) -> Result<INodeNum>;
    fn open(&mut self, inode: INodeNum, fd: ProcessFileDescriptor) -> Result<()>;
    fn create(&mut self, parent: INodeNum, name: &Path, fd: ProcessFileDescriptor) -> Result<()>;
    fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()>;
    fn read(&mut self, fd: ProcessFileDescriptor, offset: u64, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, fd: ProcessFileDescriptor, offset: u64, buf: &[u8]) -> Result<usize>;
    fn sync(&mut self) -> Result<()>;
    fn mkdir(&mut self, parent: INodeNum, name: &Path) -> Result<()>;
    fn can_be_safely_unmounted(&self) -> bool;
    fn mount(&mut self, dir: INodeNum, fs: FileSystemID) -> Result<()>;
    fn unmount(&mut self, dir: INodeNum) -> Result<()>;
    fn mount_point_at(&self, dir: INodeNum) -> Option<FileSystemID>;
    fn fstat(&mut self, fd: ProcessFileDescriptor) -> Result<FileInfo>;
    fn size_of_file(&mut self, fd: ProcessFileDescriptor) -> Result<u64>;
    fn inode_type(&mut self, inode: INodeNum) -> Result<INodeType>;
    fn read_link<'a>(&mut self, inode: INodeNum, buf: &'a mut [u8]) -> Result<Cow<'a, Path>>;
}

/// get parent directory and name of absolute path
/// e.g. /foo/bar => "/foo", "bar"
fn dirname_and_filename(path: &Path) -> (&Path, &Path) {
    let Some(final_slash) = path.rfind('/') else {
        return (".", path);
    };
    let dir = if final_slash == 0 {
        "/"
    } else {
        &path[..final_slash]
    };
    let name = &path[final_slash + 1..];
    (dir, name)
}

fn dirname_of(path: &Path) -> &Path {
    dirname_and_filename(path).0
}

fn filename_of(path: &Path) -> &Path {
    dirname_and_filename(path).1
}

impl<F: FileSystem> FileSystemManagerTrait for FileSystemManager<F> {
    fn root(&self) -> INodeNum {
        self.fs.root()
    }
    fn mount_point(&self) -> Option<(FileSystemID, INodeNum)> {
        self.mount_point
    }
    fn open(&mut self, inode: INodeNum, fd: ProcessFileDescriptor) -> Result<()> {
        let handle = self.fs.open(inode)?;
        self.open_file_handle(fd, handle)
    }
    fn create(&mut self, parent: INodeNum, name: &Path, fd: ProcessFileDescriptor) -> Result<()> {
        if name.is_empty() || name == "." || name == ".." {
            // e.g. create("foo/"), create("foo/."), create("foo/..")
            return Err(Error::IsDirectory);
        }
        let mut dir = self.temp_open(parent)?;
        let file = self.fs.create(&mut dir.handle, name);
        self.temp_close(dir);
        let file = file?;
        // add file to directory entry cache
        if let Some(dir) = self.directories.get_mut(&parent) {
            dir.add(file.inode(), INodeType::File, name);
        }
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
        self.open_files.is_empty() && self.mount_count == 0
    }
    fn sync(&mut self) -> Result<()> {
        self.fs.sync()
    }
    fn mkdir(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        if name.is_empty() || name == "." || name == ".." {
            // e.g. mkdir("/foo/"), where /foo exists.
            return Err(Error::Exists);
        }
        let mut parent_dir = self.temp_open(parent)?;
        let result = self.fs.mkdir(&mut parent_dir.handle, name);
        self.temp_close(parent_dir);
        let inode = result?;
        self.directories
            .get_mut(&parent)
            .unwrap()
            .add(inode, INodeType::Directory, name);
        self.directories.insert(
            inode,
            Directory {
                entries: Some(DirEntries::new()),
                mount: None,
                parent,
            },
        );
        Ok(())
    }
    fn read(&mut self, fd: ProcessFileDescriptor, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let handle = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        self.fs.read(handle, offset, buf)
    }
    fn write(&mut self, fd: ProcessFileDescriptor, offset: u64, buf: &[u8]) -> Result<usize> {
        let handle = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        self.fs.write(handle, offset, buf)
    }
    fn fstat(&mut self, fd: ProcessFileDescriptor) -> Result<FileInfo> {
        let handle = self.open_files.get(&fd).ok_or(Error::BadFd)?;
        self.fs.stat(handle)
    }
    fn size_of_file(&mut self, fd: ProcessFileDescriptor) -> Result<u64> {
        Ok(self.fstat(fd)?.size)
    }
    fn mount(&mut self, dir: INodeNum, fs: FileSystemID) -> Result<()> {
        // ensure directory entries are in cache
        let _ = self.lookup(dir, "x");
        let dir = self.directories.get_mut(&dir).ok_or(Error::NotDirectory)?;
        if !dir.is_empty() || dir.mount.is_some() {
            return Err(Error::NotEmpty);
        }
        dir.mount = Some(fs);
        self.mount_count += 1;
        Ok(())
    }
    fn unmount(&mut self, dir: INodeNum) -> Result<()> {
        let dir = self.directories.get_mut(&dir).ok_or(Error::NotDirectory)?;
        if dir.mount.is_none() {
            return Err(Error::NotMounted);
        }
        dir.mount = None;
        self.mount_count -= 1;
        Ok(())
    }
    fn mount_point_at(&self, dir: INodeNum) -> Option<FileSystemID> {
        self.directories.get(&dir).and_then(|dir| dir.mount)
    }
    fn lookup(&mut self, dir_inode: INodeNum, name: &Path) -> Result<INodeNum> {
        if name.is_empty() || name == "." {
            return Ok(dir_inode);
        }
        let dir = self
            .directories
            .get_mut(&dir_inode)
            .ok_or(Error::NotDirectory)?;
        if name == ".." {
            return Ok(dir.parent);
        }
        let mut new_directories = vec![];
        if dir.entries.is_none() {
            // can't use self.temp_open here due to borrowing rules
            let mut handle = temp_open(&mut self.fs, dir_inode)?;
            // TODO: consider converting entries to a BTreeMap.
            let entries = self.fs.readdir(&mut handle.handle);
            temp_close(&mut self.fs, handle, &self.open_file_count);
            let entries = entries?;
            for entry in &entries {
                if entry.r#type == INodeType::Directory {
                    new_directories.push(entry.inode);
                }
            }
            dir.entries = Some(entries);
        }
        // dir.entries should now definitely be set
        let entries = dir.entries.as_ref().unwrap();
        let result = entries
            .into_iter()
            .find_map(|e| if e.name == name { Some(e.inode) } else { None })
            .ok_or(Error::NotFound);
        for child_dir in new_directories {
            // make note of child's parent here
            // (needed so that we can resolve .. in paths)
            self.directories.insert(
                child_dir,
                Directory {
                    parent: dir_inode,
                    entries: None,
                    mount: None,
                },
            );
        }
        result
    }
    fn read_link<'a>(&mut self, inode: INodeNum, buf: &'a mut [u8]) -> Result<Cow<'a, Path>> {
        let mut handle = self.temp_open(inode)?;
        let result = self.fs.stat(&handle.handle).and_then(|st| {
            if st.r#type != INodeType::Link {
                return Err(Error::NotLink);
            }
            if buf.len() as u64 >= st.size {
                let s = self
                    .fs
                    .readlink(&mut handle.handle, buf)?
                    .expect("link size changed mysteriously");
                Ok(Cow::Borrowed(s))
            } else {
                if st.size > isize::MAX as u64 {
                    // enormous link. will probably never happen.
                    return Err(Error::IO("symlink too large".into()));
                }
                let mut buf = vec![0u8; st.size as usize];
                let s = self
                    .fs
                    .readlink(&mut handle.handle, &mut buf[..])?
                    .expect("link size changed mysteriously");
                let len = s.len();
                buf.truncate(len);
                let string = String::from_utf8(buf)
                    .expect("filesystem gave us bad UTF-8 in a str! terrible!!");
                Ok(Cow::Owned(string))
            }
        });
        self.temp_close(handle);
        result
    }
    fn inode_type(&mut self, inode: INodeNum) -> Result<INodeType> {
        let handle = self.temp_open(inode)?;
        let st = self.fs.stat(&handle.handle);
        self.temp_close(handle);
        Ok(st?.r#type)
    }
}

pub type FileSystemID = u16;

#[derive(Debug)]
enum OpenFile {
    /// regular file
    Regular { fs: FileSystemID, offset: u64 },
    /// standard output
    StdOut,
    /// /dev/null (discards reads/writes)
    Null,
}

// wrapper around an array of filesystems for convenience
struct FileSystemList([Option<Box<dyn FileSystemManagerTrait>>; MAX_MOUNT_POINTS as usize]);

impl FileSystemList {
    const fn new() -> Self {
        Self([const { None }; MAX_MOUNT_POINTS as usize])
    }
    /// get reference to filesystem with ID
    ///
    /// panics if id is invalid.
    fn get(&self, id: FileSystemID) -> &dyn FileSystemManagerTrait {
        self.0[id as usize]
            .as_ref()
            .expect("bad filesystem ID")
            .as_ref()
    }
    /// get mutable reference to filesystem with ID
    ///
    /// panics if id is invalid.
    fn get_mut(&mut self, id: FileSystemID) -> &mut dyn FileSystemManagerTrait {
        self.0[id as usize]
            .as_mut()
            .expect("bad filesystem ID")
            .as_mut()
    }
    fn add<F: FileSystem + 'static>(
        &mut self,
        fs: F,
        mount_point: Option<(FileSystemID, INodeNum)>,
    ) -> Result<FileSystemID> {
        let mut new_fs = None;
        for id in 0..MAX_MOUNT_POINTS as usize {
            if self.0[id].is_none() {
                self.0[id] = Some(Box::new(FileSystemManager::new(fs, mount_point)));
                new_fs = Some(id as FileSystemID);
                break;
            }
        }
        let Some(new_fs) = new_fs else {
            // Maybe this isn't the best error to return here?
            // Seems unlikely that this would happen in any case.
            return Err(Error::NoSpace);
        };
        Ok(new_fs)
    }
    fn remove(&mut self, id: FileSystemID) {
        self.0[id as usize] = None;
    }
}

pub struct RootFileSystem {
    file_systems: FileSystemList,
    root_mount: Option<FileSystemID>,
    open_files: BTreeMap<ProcessFileDescriptor, OpenFile>,
}

impl RootFileSystem {
    pub const fn new() -> Self {
        Self {
            file_systems: FileSystemList::new(),
            root_mount: None,
            open_files: BTreeMap::new(),
        }
    }
    fn resolve_path_relative_to(
        &mut self,
        cwd: (FileSystemID, INodeNum),
        path: &Path,
        level_of_links: usize,
    ) -> Result<(FileSystemID, INodeNum)> {
        if level_of_links > MAX_LEVEL_OF_LINKS {
            return Err(Error::TooManyLevelsOfLinks);
        }
        let mut fs_id = self.root_mount.ok_or(Error::NotFound)?;
        let mut fs_root = self.file_systems.get(fs_id).root();
        let mut inode;
        if path.starts_with('/') {
            inode = fs_root;
        } else {
            (fs_id, inode) = cwd;
            fs_root = self.file_systems.get(fs_id).root();
        }
        let mut link_buf = [0; 256];
        for component in path.split('/') {
            if component.is_empty() || component == "." {
                continue;
            }
            if component == ".." && inode == fs_root {
                // .. from root of filesystem
                // escape to parent filesystem, or do nothing if at /
                if let Some((parent_fs, ino)) = self.file_systems.get(fs_id).mount_point() {
                    fs_id = parent_fs;
                    fs_root = self.file_systems.get(fs_id).root();
                    inode = ino;
                }
                continue;
            }
            let fs = self.file_systems.get_mut(fs_id);
            inode = fs.lookup(inode, component)?;
            if let Some(child_fs) = fs.mount_point_at(inode) {
                // enter mount
                fs_id = child_fs;
                fs_root = self.file_systems.get(fs_id).root();
                inode = fs_root;
                continue;
            }
            match fs.read_link(inode, &mut link_buf) {
                Err(Error::NotLink) => {}
                Ok(link_dest) => {
                    (fs_id, inode) = self.resolve_path_relative_to(
                        (fs_id, inode),
                        link_dest.as_ref(),
                        level_of_links + 1,
                    )?;
                }
                Err(e) => return Err(e),
            }
        }
        Ok((fs_id, inode))
    }
    /// Determine which filesystem a path belongs to, and inode number in the filesystem.
    fn resolve_path(
        &mut self,
        process: &ProcessControlBlock,
        path: &Path,
    ) -> Result<(FileSystemID, INodeNum)> {
        self.resolve_path_relative_to(process.cwd, path, 0)
    }
    pub fn get_root(&self) -> Result<(FileSystemID, INodeNum)> {
        let root_fs = self.root_mount.ok_or(Error::NotFound)?;
        Ok((root_fs, self.file_systems.get(root_fs).root()))
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
    pub fn mount<F: FileSystem + 'static>(
        &mut self,
        process: &ProcessControlBlock,
        path: &Path,
        fs: F,
    ) -> Result<()> {
        let (parent_fs, inode) = self.resolve_path(process, path)?;
        let new_fs = self.file_systems.add(fs, Some((parent_fs, inode)))?;
        let result = self.file_systems.get_mut(parent_fs).mount(inode, new_fs);
        if result.is_err() {
            self.file_systems.remove(new_fs);
        }
        result
    }
    pub fn unmount(&mut self, process: &ProcessControlBlock, path: &Path) -> Result<()> {
        let (child_fs_id, _) = self.resolve_path(process, path)?;
        let Some((parent_fs_id, inode)) = self.file_systems.get(child_fs_id).mount_point() else {
            // ordinary processes probably shouldn't unmount /
            return Err(Error::FileSystemInUse);
        };
        let parent_fs = self.file_systems.get_mut(parent_fs_id);
        let child_fs_id = parent_fs.mount_point_at(inode).ok_or(Error::NotFound)?;
        let fs = self.file_systems.get_mut(child_fs_id);
        if !fs.can_be_safely_unmounted() {
            return Err(Error::FileSystemInUse);
        }
        fs.sync()?;
        self.file_systems.remove(child_fs_id);
        let parent_fs = self.file_systems.get_mut(parent_fs_id);
        // parent_fs.unmount should only fail if inode isn't a mount point, but we checked that already.
        parent_fs.unmount(inode).unwrap();
        Ok(())
    }
    pub fn mount_root<F: FileSystem + 'static>(&mut self, fs: F) -> Result<()> {
        if self.root_mount.is_some() {
            return Err(Error::NotEmpty);
        }
        let new_fs = self.file_systems.add(fs, None)?;
        self.root_mount = Some(new_fs);
        Ok(())
    }
    pub fn open(
        &mut self,
        process: &ProcessControlBlock,
        path: &Path,
        mode: Mode,
    ) -> Result<FileDescriptor> {
        let (fs, inode) = match mode {
            Mode::ReadWrite => self.resolve_path(process, path)?,
            Mode::CreateReadWrite => self.resolve_path(process, dirname_of(path))?,
        };
        let fd = self.new_fd(process.pid, OpenFile::Regular { fs, offset: 0 })?;
        let fs = self.file_systems.get_mut(fs);
        let result = match mode {
            Mode::ReadWrite => fs.open(inode, fd),
            Mode::CreateReadWrite => fs.create(inode, filename_of(path), fd),
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
            let fs = self.file_systems.get_mut(*fs);
            result = fs.close(fd);
        }
        // don't need to do anything for non-regular files
        self.open_files.remove(&fd);
        result
    }
    pub fn mkdir(&mut self, process: &ProcessControlBlock, path: &Path) -> Result<()> {
        let (parent, name) = dirname_and_filename(path);
        let (fs, parent) = self.resolve_path(process, parent)?;
        let fs = self.file_systems.get_mut(fs);
        fs.mkdir(parent, name)
    }
    pub fn read(&mut self, fd: ProcessFileDescriptor, buf: &mut [u8]) -> Result<usize> {
        let file_info = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        match file_info {
            OpenFile::Regular { fs, offset } => {
                let fs = self.file_systems.get_mut(*fs);
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
                let fs = self.file_systems.get_mut(*fs);
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
                        let fs = self.file_systems.get_mut(*fs);
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
    pub fn chdir(&mut self, process: &mut ProcessControlBlock, path: &Path) -> Result<()> {
        let (fs_id, inode) = self.resolve_path(process, path)?;
        if self.file_systems.get_mut(fs_id).inode_type(inode)? != INodeType::Directory {
            return Err(Error::NotDirectory);
        }
        process.cwd = (fs_id, inode);
        if path.starts_with('/') {
            // chdir to absolute path
            process.cwd_path.clear();
        }
        for component in path.split('/') {
            if component == "." || component.is_empty() {
                continue;
            }
            if component == ".." {
                let last_slash = process
                    .cwd_path
                    .rfind('/')
                    .expect("cwd should be an absolute path");
                if last_slash == 0 {
                    // cwd_path/.. is just /
                    process.cwd_path.truncate(1);
                } else {
                    // remove final component in cwd_path
                    process.cwd_path.truncate(last_slash);
                }
                continue;
            }
            process.cwd_path.push('/');
            process.cwd_path.push_str(component);
        }
        Ok(())
    }
    pub fn fstat(&mut self, fd: ProcessFileDescriptor) -> Result<FileInfo> {
        let file = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        if let OpenFile::Regular { fs, .. } = file {
            self.file_systems.get_mut(*fs).fstat(fd)
        } else {
            Err(Error::NotFound)
        }
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
    // fake PCB for testing
    const PCB: ProcessControlBlock = ProcessControlBlock {
        pid: 0,
        child_tids: vec![],
        wait_list: vec![],
        exit_code: None,
    };
    // open file for fake PID of 1 for testing
    fn open(root: &mut RootFileSystem, path: &Path, mode: Mode) -> Result<ProcessFileDescriptor> {
        let pid = 0;
        let fd = root.open(&PCB, path, mode)?;
        Ok(ProcessFileDescriptor { fd, pid })
    }
    #[test]
    fn test_one_filesystem_simple() {
        let mut root = RootFileSystem::new();
        let fs = TempFS::new();
        root.mount_root(fs).unwrap();
        let file = open(&mut root, "/foo", Mode::CreateReadWrite).unwrap();
        assert_eq!(root.write(file, b"test data").unwrap(), 9);
        root.close(file).unwrap();
        let file = open(&mut root, "/foo", Mode::ReadWrite).unwrap();
        let mut buf = [0; 10];
        assert_eq!(root.read(file, &mut buf).unwrap(), 9);
        assert_eq!(&buf, b"test data\0");
        root.close(file).unwrap();
    }
    #[test]
    fn test_multiple_filesystems_simple() {
        let mut root = RootFileSystem::new();
        let fs = TempFS::new();
        root.mount_root(fs).unwrap();
        let fs2 = TempFS::new();
        root.mkdir(&PCB, "/2").unwrap();
        root.mount(&PCB, "/2", fs2).unwrap();
        let fs3 = TempFS::new();
        root.mkdir(&PCB, "/2/3").unwrap();
        root.mount(&PCB, "/2/3", fs3).unwrap();
        for path in ["/foo", "/2/foo", "/2/3/foo"] {
            let file = open(&mut root, path, Mode::CreateReadWrite).unwrap();
            // we shouldn't be allowed to unmount the FS file is contained in while it's open
            assert!(matches!(
                root.unmount(&PCB, dirname_and_filename(path).0),
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
        assert!(matches!(
            root.unmount(&PCB, "/2"),
            Err(Error::FileSystemInUse)
        ));
        root.unmount(&PCB, "/2/3").unwrap();
        root.unmount(&PCB, "/2").unwrap();
    }
}
