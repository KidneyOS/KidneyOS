use crate::fs::{FileDescriptor, ProcessFileDescriptor};
use crate::mem::vma::{VMAInfo, VMA};
use crate::system::{running_process, unwrap_system};
use crate::threading::{process::Pid, thread_control_block::ProcessControlBlock};
use crate::user_program::syscall::Dirent;
use crate::vfs::{
    Error, FileHandle, FileInfo, FileSystem, INodeNum, INodeType, OwnedDirEntry, OwnedPath, Path,
    Result,
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
use core::mem::{align_of, size_of};
use core::num::NonZeroUsize;
use kidneyos_shared::mem::PAGE_FRAME_SIZE;

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
    /// map from directory entry IDs to directory entries
    ///
    /// We need this IDs in order for readdir to work properly. Specifically we have to guarantee that
    /// if entries are added/removed from the directory between calls to getdents, then other unrelated entries
    /// are never skipped over or repeated.
    ///
    /// We accomplish this by assigning an ID to each directory entry. These IDs are always increasing,
    /// and the "offset" member of a directory fd is the next ID it will read.
    ///
    /// If this is `None`, that means the directory entries haven't been scanned yet.
    /// This scanning is done in [`FileSystemManagerTrait`].
    entries: Option<BTreeMap<u64, OwnedDirEntry>>,
    /// map from paths to directory entry IDs
    lookup: BTreeMap<OwnedPath, u64>,
    /// next directory entry ID to hand out
    id: u64,
    /// inode number of parent directory (needed to resolve ..)
    parent: INodeNum,
    /// File system that is mounted to this directory, if any.
    mount: Option<FileSystemID>,
}

impl Directory {
    fn new(parent: INodeNum) -> Self {
        Directory {
            entries: None,
            mount: None,
            parent,
            id: 0,
            lookup: BTreeMap::new(),
        }
    }
    fn empty(parent: INodeNum) -> Self {
        let mut dir = Self::new(parent);
        dir.entries = Some(BTreeMap::new());
        dir
    }
    fn add(&mut self, inode: INodeNum, r#type: INodeType, name: &Path) {
        let id = self.id;
        self.id += 1;
        self.entries
            .as_mut()
            .expect("Directory::add called before directory entries were scanned")
            .insert(
                id,
                OwnedDirEntry {
                    r#type,
                    inode,
                    name: Cow::Owned(name.into()),
                },
            );
        self.lookup.insert(name.into(), id);
    }
    fn remove(&mut self, name: &Path) {
        let entries = self
            .entries
            .as_mut()
            .expect("Directory::remove called before directory entries were scanned");
        if let Some(id) = self.lookup.remove(name) {
            entries.remove(&id);
        }
    }
    fn lookup_inode(&self, name: &Path) -> Option<INodeNum> {
        Some(
            self.entries
                .as_ref()
                .expect("Directory::lookup_inode called before directory entries were scanned")
                .get(self.lookup.get(name)?)?
                .inode,
        )
    }
    fn is_empty(&self) -> bool {
        self.entries
            .as_ref()
            .expect("Directory::is_empty called before directory entries were scanned")
            .is_empty()
    }

    /// # Safety
    ///
    /// See [`FileSystemManagerTrait::getdents`].
    unsafe fn getdents(
        &self,
        offset: &mut u64,
        output: *mut Dirent,
        mut size: usize,
    ) -> Result<usize> {
        let entries = self
            .entries
            .as_ref()
            .expect("Directory::getdents called before directory entries were scanned");
        let mut bytes_read = 0;
        let mut output: *mut u8 = output.cast();
        for entry in entries.range(*offset..) {
            let off = *entry.0;
            let r#type = entry.1.r#type;
            let inode = entry.1.inode;
            let name = &entry.1.name;
            let required_bytes = size_of::<Dirent>() + name.len() + 1;
            let dirent_align = align_of::<Dirent>();
            // round up to dirent alignment
            let required_bytes = required_bytes.div_ceil(dirent_align) * dirent_align;
            if size < required_bytes {
                break;
            }
            let Ok(reclen) = u16::try_from(required_bytes) else {
                return Err(Error::IO("file name too long".into()));
            };
            let dirent = Dirent {
                offset: off as i64,
                inode,
                reclen,
                r#type: r#type.to_u8(),
                name: [],
            };
            unsafe {
                let dirent_ptr: *mut Dirent = output.cast();
                assert!(dirent_ptr.is_aligned());
                dirent_ptr.write(dirent);
                let name_ptr: *mut u8 = dirent_ptr
                    .cast::<u8>()
                    .add(core::mem::offset_of!(Dirent, name));
                name_ptr.copy_from_nonoverlapping(name.as_ptr(), name.len());
                name_ptr.add(name.len()).write(0); // null terminator
            }
            size -= required_bytes;
            output = output.add(required_bytes);
            bytes_read += required_bytes;
            *offset = off + 1;
        }
        Ok(bytes_read)
    }
}

/// Manages a single file system
struct FileSystemManager<F: FileSystem> {
    fs: F,
    /// Location where this file system is mounted, or `None` if this is the root file system.
    mount_point: Option<(FileSystemID, INodeNum)>,
    /// Number of open files pointing to inodes.
    open_file_count: BTreeMap<INodeNum, NonZeroUsize>,
    /// VFS file handles for each file descriptor
    open_files: BTreeMap<ProcessFileDescriptor, F::FileHandle>,
    /// Cached directory entries
    directories: BTreeMap<INodeNum, Directory>,
    /// Number of mount points in this file system.
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

/// Temporarily open a file.
///
/// The return value *must not be dropped* --- it should instead be passed to `temp_close`.
///
/// (This is difficult to do with a destructor because of borrowing rules)
fn temp_open<F: FileSystem>(fs: &mut F, inode: INodeNum) -> Result<TempOpen<F>> {
    let handle = fs.open(inode)?;
    Ok(TempOpen { handle })
}

/// Close a file opened with [`temp_open`].
fn temp_close<F: FileSystem>(
    fs: &mut F,
    file: TempOpen<F>,
    open_file_count: &BTreeMap<INodeNum, NonZeroUsize>,
) {
    let inode = file.handle.inode();
    if !open_file_count.contains_key(&inode) {
        fs.release(inode);
    }
    // prevent drop from running
    core::mem::forget(file);
}

impl<F: FileSystem + 'static> FileSystemManager<F> {
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
        me.directories.insert(root_ino, Directory::new(root_ino));
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
        self.inc_ref(handle.inode());
        let _prev = self.open_files.insert(fd, handle);
        debug_assert!(_prev.is_none(), "duplicate fd");
        Ok(())
    }
    fn create_handle(&mut self, parent: INodeNum, name: &Path) -> Result<F::FileHandle> {
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
        Ok(file)
    }
}

/// Unfortunately `FileSystemManager<dyn FileSystem>` doesn't work (we'd have to specify the
/// FileHandle type). So we need a new trait to be able to create dynamic objects
/// which can use different file systems.
trait FileSystemManagerTrait: 'static + Send + Sync {
    /// Get root inode
    fn root(&self) -> INodeNum;
    /// Get inode number for file descriptor
    fn inode_of(&self, fd: ProcessFileDescriptor) -> Result<INodeNum>;
    /// Get location where this FS is mounted, or `None` if this is the root FS.
    fn mount_point(&self) -> Option<(FileSystemID, INodeNum)>;
    fn lookup(&mut self, dir: INodeNum, entry: &Path) -> Result<INodeNum>;
    fn open(&mut self, inode: INodeNum, fd: ProcessFileDescriptor) -> Result<()>;
    fn create(&mut self, parent: INodeNum, name: &Path, fd: ProcessFileDescriptor) -> Result<()>;
    fn create_raw(&mut self, parent: INodeNum, name: &Path) -> Result<INodeNum>;
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
    fn unlink(&mut self, parent: INodeNum, name: &Path) -> Result<()>;
    fn rmdir(&mut self, parent: INodeNum, name: &Path) -> Result<()>;
    fn link(&mut self, source: INodeNum, parent: INodeNum, name: &Path) -> Result<()>;
    fn symlink(&mut self, link: &Path, parent: INodeNum, name: &Path) -> Result<()>;
    fn rename(
        &mut self,
        source_parent: INodeNum,
        source_name: &Path,
        dest_parent: INodeNum,
        dest_name: &Path,
    ) -> Result<()>;
    /// Read directory entries into `entries`.
    /// Returns the number of bytes read.
    /// Advances `offset` past the directory entries read.
    ///
    /// # Safety
    ///
    /// entries must be valid for writing up to `size` bytes.
    unsafe fn getdents(
        &mut self,
        dir: ProcessFileDescriptor,
        offset: &mut u64,
        entries: *mut Dirent,
        size: usize,
    ) -> Result<usize>;
    fn ftruncate(&mut self, file: ProcessFileDescriptor, size: u64) -> Result<()>;
    /// increase reference count of inode (pretend there is an extra open file to it)
    fn inc_ref(&mut self, inode: INodeNum);
    /// decrease reference count of inode (pretend there is one fewer open file to it)
    fn dec_ref(&mut self, inode: INodeNum);
    /// Read bytes directly from a file
    fn read_raw(&mut self, inode: INodeNum, offset: u64, buf: &mut [u8]) -> Result<usize>;
    /// Write bytes directly to a file
    fn write_raw(&mut self, inode: INodeNum, offset: u64, buf: &[u8]) -> Result<()>;
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

impl<F: 'static + FileSystem> FileSystemManagerTrait for FileSystemManager<F> {
    fn root(&self) -> INodeNum {
        self.fs.root()
    }
    fn inode_of(&self, fd: ProcessFileDescriptor) -> Result<INodeNum> {
        Ok(self.open_files.get(&fd).ok_or(Error::BadFd)?.inode())
    }
    fn mount_point(&self) -> Option<(FileSystemID, INodeNum)> {
        self.mount_point
    }
    fn open(&mut self, inode: INodeNum, fd: ProcessFileDescriptor) -> Result<()> {
        let handle = self.fs.open(inode)?;
        self.open_file_handle(fd, handle)
    }
    fn create_raw(&mut self, parent: INodeNum, name: &Path) -> Result<INodeNum> {
        let file = self.create_handle(parent, name)?;
        self.inc_ref(file.inode());
        Ok(file.inode())
    }
    fn create(&mut self, parent: INodeNum, name: &Path, fd: ProcessFileDescriptor) -> Result<()> {
        let file = self.create_handle(parent, name)?;
        self.open_file_handle(fd, file)
    }
    fn close(&mut self, fd: ProcessFileDescriptor) -> Result<()> {
        let handle = self.open_files.remove(&fd).ok_or(Error::BadFd)?;
        self.dec_ref(handle.inode());
        Ok(())
    }
    fn can_be_safely_unmounted(&self) -> bool {
        self.open_file_count.is_empty() && self.mount_count == 0
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
        self.directories.insert(inode, Directory::empty(parent));
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
        let mut new_directories = vec![];
        let dir = self
            .directories
            .get_mut(&dir_inode)
            .ok_or(Error::NotDirectory)?;
        if name == ".." {
            return Ok(dir.parent);
        }
        if dir.entries.is_none() {
            // can't use self.temp_open here due to borrowing rules
            let mut handle = temp_open(&mut self.fs, dir_inode)?;
            let entries = self.fs.readdir(&mut handle.handle);
            temp_close(&mut self.fs, handle, &self.open_file_count);
            let entries = entries?;
            for entry in &entries {
                if entry.r#type == INodeType::Directory {
                    new_directories.push(entry.inode);
                }
            }
            dir.entries = Some(BTreeMap::new());
            for entry in &entries {
                dir.add(entry.inode, entry.r#type, &entry.name);
            }
        }
        let inode = dir.lookup_inode(name).ok_or(Error::NotFound)?;
        for child_dir in new_directories {
            // make note of child's parent here
            // (needed so that we can resolve .. in paths)
            self.directories
                .insert(child_dir, Directory::new(dir_inode));
        }
        Ok(inode)
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
    fn unlink(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        let dir = self.directories.get_mut(&parent).ok_or(Error::NotFound)?;
        let mut handle = temp_open(&mut self.fs, parent)?;
        let result = self.fs.unlink(&mut handle.handle, name);
        temp_close(&mut self.fs, handle, &self.open_file_count);
        dir.remove(name);
        result
    }
    fn rmdir(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        let dir = self.directories.get_mut(&parent).ok_or(Error::NotFound)?;
        let mut handle = temp_open(&mut self.fs, parent)?;
        let result = self.fs.rmdir(&mut handle.handle, name);
        temp_close(&mut self.fs, handle, &self.open_file_count);
        dir.remove(name);
        result
    }
    unsafe fn getdents(
        &mut self,
        dir: ProcessFileDescriptor,
        offset: &mut u64,
        entries: *mut Dirent,
        size: usize,
    ) -> Result<usize> {
        let inode = self.open_files.get(&dir).ok_or(Error::BadFd)?.inode();
        // ensure directory entries are loaded
        let _ = self.lookup(inode, "x");
        let dir = self.directories.get(&inode).ok_or(Error::NotDirectory)?;
        if dir.entries.is_none() {
            return Err(Error::IO("failed to read directory entries".into()));
        }
        dir.getdents(offset, entries, size)
    }
    fn link(&mut self, source: INodeNum, parent: INodeNum, name: &Path) -> Result<()> {
        if name.is_empty() || name == "." || name == ".." {
            return Err(Error::Exists);
        }
        let mut source_handle = temp_open(&mut self.fs, source)?;
        let source_info = self.fs.stat(&source_handle.handle)?;
        if source_info.r#type == INodeType::Directory {
            return Err(Error::IsDirectory);
        }
        let parent_handle = temp_open(&mut self.fs, parent);
        let result = parent_handle.and_then(|mut parent_handle| {
            let r = self
                .fs
                .link(&mut source_handle.handle, &mut parent_handle.handle, name);
            temp_close(&mut self.fs, parent_handle, &self.open_file_count);
            r
        });
        temp_close(&mut self.fs, source_handle, &self.open_file_count);
        result?;
        self.directories
            .get_mut(&parent)
            .unwrap()
            .add(source, INodeType::File, name);
        Ok(())
    }
    fn symlink(&mut self, link: &Path, parent: INodeNum, name: &Path) -> Result<()> {
        if name.is_empty() || name == "." || name == ".." {
            return Err(Error::Exists);
        }
        let mut parent_handle = temp_open(&mut self.fs, parent)?;
        let result = self.fs.symlink(link, &mut parent_handle.handle, name);
        temp_close(&mut self.fs, parent_handle, &self.open_file_count);
        let symlink_inode = result?;
        self.directories
            .get_mut(&parent)
            .unwrap()
            .add(symlink_inode, INodeType::Link, name);
        Ok(())
    }
    fn rename(
        &mut self,
        source_parent: INodeNum,
        source_name: &Path,
        dest_parent: INodeNum,
        dest_name: &Path,
    ) -> Result<()> {
        // perform   rename("a", "b")
        // by doing  link("a", "b"), unlink("a")
        let source_inode = self.lookup(source_parent, source_name)?;
        self.link(source_inode, dest_parent, dest_name)?;
        self.unlink(source_parent, source_name)
    }
    fn ftruncate(&mut self, fd: ProcessFileDescriptor, size: u64) -> Result<()> {
        let handle = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        self.fs.truncate(handle, size)
    }
    fn inc_ref(&mut self, inode: INodeNum) {
        match self.open_file_count.entry(inode) {
            BTreeMapEntry::Occupied(mut o) => {
                let count = o.get_mut();
                *count = count
                    .checked_add(1)
                    .expect("shouldn't overflow usize (each open file requires ≥1 byte of memory)");
            }
            BTreeMapEntry::Vacant(v) => {
                v.insert(NonZeroUsize::new(1).unwrap());
            }
        }
    }
    fn dec_ref(&mut self, inode: INodeNum) {
        let Some(ref_count) = self.open_file_count.get_mut(&inode) else {
            return;
        };
        match NonZeroUsize::new(ref_count.get() - 1) {
            Some(n) => {
                // other open files with the same inode still exist
                *ref_count = n;
            }
            None => {
                // all open files to this inode have been closed
                self.open_file_count.remove(&inode);
                self.fs.release(inode);
            }
        }
    }
    fn read_raw(&mut self, inode: INodeNum, mut offset: u64, mut buf: &mut [u8]) -> Result<usize> {
        let mut handle = self.temp_open(inode)?;
        let mut bytes_read = 0;
        while !buf.is_empty() {
            let n = self.fs.read(&mut handle.handle, offset, buf);
            match n {
                Ok(0) => break,
                Ok(n) => {
                    bytes_read += n;
                    offset += n as u64;
                    buf = &mut buf[n..];
                }
                Err(e) => {
                    self.temp_close(handle);
                    return Err(e);
                }
            }
        }
        self.temp_close(handle);
        Ok(bytes_read)
    }
    fn write_raw(&mut self, inode: INodeNum, mut offset: u64, mut buf: &[u8]) -> Result<()> {
        let mut handle = self.temp_open(inode)?;
        while !buf.is_empty() {
            let n = self.fs.write(&mut handle.handle, offset, buf);
            match n {
                Ok(n) => {
                    assert!(n > 0, "zero-sized write probably shouldn't happen");
                    offset += n as u64;
                    buf = &buf[n..];
                }
                Err(e) => {
                    self.temp_close(handle);
                    return Err(e);
                }
            }
        }
        self.temp_close(handle);
        Ok(())
    }
}

pub type FileSystemID = u16;

/// Metadata for an open file
#[derive(Debug)]
enum OpenFile {
    /// regular file/directory
    Regular {
        fs: FileSystemID,
        offset: u64,
        is_dir: bool,
    },
    /// standard output
    StdOut,
    /// `/dev/null` (discards reads/writes)
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
        match self.0[id as usize].as_ref() {
            Some(fs) => fs.as_ref(),
            None => panic!("bad filesystem ID: {id}"),
        }
    }
    /// get mutable reference to filesystem with ID
    ///
    /// panics if id is invalid.
    fn get_mut(&mut self, id: FileSystemID) -> &mut dyn FileSystemManagerTrait {
        match self.0[id as usize].as_mut() {
            Some(fs) => fs.as_mut(),
            None => panic!("bad filesystem ID: {id}"),
        }
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
    fn iter_mut(
        &mut self,
    ) -> impl '_ + Iterator<Item = &'_ mut (dyn 'static + FileSystemManagerTrait)> {
        self.0
            .iter_mut()
            .filter_map(move |fs| Some(fs.as_mut()?.as_mut()))
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
                // note: don't continue; here, we want to go to the parent folder in the parent file system
            }
            let fs = self.file_systems.get_mut(fs_id);
            let child_inode = fs.lookup(inode, component)?;
            if let Some(child_fs) = fs.mount_point_at(child_inode) {
                // enter mount
                fs_id = child_fs;
                fs_root = self.file_systems.get(fs_id).root();
                inode = fs_root;
                continue;
            }
            match fs.read_link(child_inode, &mut link_buf) {
                Err(Error::NotLink) => {
                    inode = child_inode;
                }
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
        let fd = self.new_fd(
            process.pid,
            OpenFile::Regular {
                fs,
                offset: 0,
                is_dir: false,
            },
        )?;
        let fs = self.file_systems.get_mut(fs);
        let result = match mode {
            Mode::ReadWrite => {
                fs.open(inode, fd).and_then(|()| {
                    if fs.fstat(fd)?.r#type == INodeType::Directory {
                        // set is_dir to true in open file info
                        let OpenFile::Regular { is_dir, .. } =
                            self.open_files.get_mut(&fd).unwrap()
                        else {
                            panic!();
                        };
                        *is_dir = true;
                    }
                    Ok(())
                })
            }
            Mode::CreateReadWrite => fs.create(inode, filename_of(path), fd),
        };
        if let Err(e) = result {
            self.open_files.remove(&fd);
            return Err(e);
        }
        Ok(fd.fd)
    }
    /// Increments reference count to inode and returns the filesystem ID and inode number.
    pub fn open_raw_file(
        &mut self,
        cwd: Option<(FileSystemID, INodeNum)>,
        path: &Path,
        mode: Mode,
    ) -> Result<(FileSystemID, INodeNum)> {
        let cwd = cwd.unwrap_or(self.get_root()?);
        match mode {
            Mode::ReadWrite => {
                let (fs_id, inode) = self.resolve_path_relative_to(cwd, path, 0)?;
                let fs = self.file_systems.get_mut(fs_id);
                fs.inc_ref(inode);
                if fs.inode_type(inode)? == INodeType::Directory {
                    return Err(Error::IsDirectory);
                }
                Ok((fs_id, inode))
            }
            Mode::CreateReadWrite => {
                let (fs_id, parent_inode) =
                    self.resolve_path_relative_to(cwd, dirname_of(path), 0)?;
                let fs = self.file_systems.get_mut(fs_id);
                let inode = fs.create_raw(parent_inode, filename_of(path))?;
                Ok((fs_id, inode))
            }
        }
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
            OpenFile::Regular { fs, offset, is_dir } => {
                if *is_dir {
                    return Err(Error::IsDirectory);
                }
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
            OpenFile::Regular { fs, offset, is_dir } => {
                if *is_dir {
                    return Err(Error::IsDirectory);
                }
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
            is_dir,
        } = file_info
        {
            let new_offset = offset
                .checked_add(match whence {
                    SeekFrom::Start => 0,
                    SeekFrom::Current => {
                        // only SEEK_SET should be used for directories
                        if *is_dir {
                            return Err(Error::IsDirectory);
                        }
                        *file_offset as i64
                    }
                    SeekFrom::End => {
                        // only SEEK_SET should be used for directories
                        if *is_dir {
                            return Err(Error::IsDirectory);
                        }
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
        if process.cwd_path != "/" {
            // decrement reference count to previous cwd
            let (prev_fs, prev_inode) = process.cwd;
            self.file_systems.get_mut(prev_fs).dec_ref(prev_inode);
        }
        let (fs_id, inode) = self.resolve_path(process, path)?;
        let fs = self.file_systems.get_mut(fs_id);
        if fs.inode_type(inode)? != INodeType::Directory {
            return Err(Error::NotDirectory);
        }
        // increment reference count to new cwd (e.g. this prevents it from being unmounted)
        fs.inc_ref(inode);

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
    pub fn unlink(&mut self, process: &ProcessControlBlock, path: &Path) -> Result<()> {
        let (dirname, filename) = dirname_and_filename(path);
        let (fs_id, inode) = self.resolve_path(process, dirname)?;
        self.file_systems.get_mut(fs_id).unlink(inode, filename)
    }
    pub fn rmdir(&mut self, process: &ProcessControlBlock, path: &Path) -> Result<()> {
        let (dirname, filename) = dirname_and_filename(path);
        let (fs_id, inode) = self.resolve_path(process, dirname)?;
        self.file_systems.get_mut(fs_id).rmdir(inode, filename)
    }
    pub fn link(
        &mut self,
        process: &ProcessControlBlock,
        source: &Path,
        dest: &Path,
    ) -> Result<()> {
        let (source_fs, inode) = self.resolve_path(process, source)?;
        let (dest_dirname, dest_filename) = dirname_and_filename(dest);
        let (parent_fs, parent_inode) = self.resolve_path(process, dest_dirname)?;
        if parent_fs != source_fs {
            return Err(Error::HardLinkBetweenFileSystems);
        }
        let fs = self.file_systems.get_mut(source_fs);
        fs.link(inode, parent_inode, dest_filename)
    }
    pub fn symlink(
        &mut self,
        process: &ProcessControlBlock,
        source: &Path,
        dest: &Path,
    ) -> Result<()> {
        let (dest_dirname, dest_filename) = dirname_and_filename(dest);
        let (parent_fs, parent_inode) = self.resolve_path(process, dest_dirname)?;
        self.file_systems
            .get_mut(parent_fs)
            .symlink(source, parent_inode, dest_filename)
    }
    pub fn rename(
        &mut self,
        process: &ProcessControlBlock,
        source: &Path,
        dest: &Path,
    ) -> Result<()> {
        let (source_dirname, source_filename) = dirname_and_filename(source);
        let (dest_dirname, dest_filename) = dirname_and_filename(dest);
        let (source_parent_fs, source_parent_inode) = self.resolve_path(process, source_dirname)?;
        let (dest_parent_fs, dest_parent_inode) = self.resolve_path(process, dest_dirname)?;
        if source_parent_fs == dest_parent_fs {
            let fs = self.file_systems.get_mut(source_parent_fs);
            fs.rename(
                source_parent_inode,
                source_filename,
                dest_parent_inode,
                dest_filename,
            )
        } else {
            // should probably handle this properly at some point…
            Err(Error::HardLinkBetweenFileSystems)
        }
    }

    /// Sync all filesystems to disk
    pub fn sync(&mut self) -> Result<()> {
        let mut result = Ok(());
        for fs in self.file_systems.iter_mut() {
            // don't break out when one fs fails -- sync as many file systems as possible
            result = result.and(fs.sync());
        }
        result
    }

    /// Read up to `size` bytes of directory entries into `output`.
    ///
    /// Returns the number of bytes read.
    ///
    /// # Safety
    ///
    /// `output` must be valid for writing up to `size` bytes.
    pub unsafe fn getdents(
        &mut self,
        fd: ProcessFileDescriptor,
        output: *mut Dirent,
        size: usize,
    ) -> Result<usize> {
        let file_info = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        match file_info {
            OpenFile::Regular {
                fs,
                offset,
                is_dir: true,
            } => {
                let fs = self.file_systems.get_mut(*fs);
                let read_count = fs.getdents(fd, offset, output, size)?;
                Ok(read_count)
            }
            _ => Err(Error::NotDirectory),
        }
    }

    pub fn ftruncate(&mut self, fd: ProcessFileDescriptor, size: u64) -> Result<()> {
        let file_info = self.open_files.get_mut(&fd).ok_or(Error::BadFd)?;
        match file_info {
            OpenFile::Regular { fs, offset, is_dir } => {
                if *is_dir {
                    return Err(Error::IsDirectory);
                }
                if *offset > size {
                    *offset = size;
                }
                let fs = self.file_systems.get_mut(*fs);
                fs.ftruncate(fd, size)
            }
            _ => Err(Error::IO("can't truncate special file".into())),
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
        if let Some(pcb) = unwrap_system().process.table.get(pid) {
            // decrement reference count to cwd
            let pcb = pcb.lock();
            let (cwd_fs, cwd_inode) = pcb.cwd;
            self.file_systems.get_mut(cwd_fs).dec_ref(cwd_inode);
            for (_addr, vma) in pcb.vmas.iter() {
                if let VMAInfo::MMap { fs, inode, .. } = vma.info() {
                    // decrease reference count to inode to let it be released.
                    self.file_systems.get_mut(*fs).dec_ref(*inode);
                }
            }
        }
    }

    pub fn inode_of(&self, fd: ProcessFileDescriptor) -> Result<(FileSystemID, INodeNum)> {
        let OpenFile::Regular { fs, .. } = self.open_files.get(&fd).ok_or(Error::BadFd)? else {
            return Err(Error::IO("can't get inode number of special file".into()));
        };
        let fs = *fs;
        Ok((fs, self.file_systems.get(fs).inode_of(fd)?))
    }

    /// Increment reference count to inode
    pub fn increment_inode_ref_count(&mut self, fs_id: FileSystemID, inode: INodeNum) {
        self.file_systems.get_mut(fs_id).inc_ref(inode);
    }

    /// Decrement reference count to inode
    pub fn decrement_inode_ref_count(&mut self, fs_id: FileSystemID, inode: INodeNum) {
        self.file_systems.get_mut(fs_id).dec_ref(inode);
    }

    /// Read bytes directly from a file using its filesystem ID and inode number.
    pub fn read_raw(
        &mut self,
        fs_id: FileSystemID,
        inode: INodeNum,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<usize> {
        self.file_systems
            .get_mut(fs_id)
            .read_raw(inode, offset, buffer)
    }
    /// Write bytes directly to a file using its filesystem ID and inode number.
    pub fn write_raw(
        &mut self,
        fs_id: FileSystemID,
        inode: INodeNum,
        offset: u64,
        buffer: &[u8],
    ) -> Result<()> {
        self.file_systems
            .get_mut(fs_id)
            .write_raw(inode, offset, buffer)
    }
    /// Map file by inode into memory
    ///
    /// Returns `Ok(false)` if there is already something mapped in `addr..addr + length`
    pub fn mmap_inode(
        &mut self,
        addr: usize,
        fs_id: FileSystemID,
        inode: INodeNum,
        length: usize,
        offset_in_pages: u32,
        writeable: bool,
    ) -> Result<bool> {
        // increase reference count to ensure that file data is kept around even if file is unlinked and all descriptors are closed.
        self.file_systems.get_mut(fs_id).inc_ref(inode);
        let pcb = running_process();
        let mut pcb = pcb.lock();
        Ok(pcb.vmas.add_vma(
            VMA::new(
                VMAInfo::MMap {
                    fs: fs_id,
                    inode,
                    offset: offset_in_pages,
                },
                length,
                writeable,
            ),
            addr,
        ))
    }

    /// Map file into memory
    ///
    /// Returns `Ok(false)` if the requested address range is unavailable.
    pub fn mmap_file(
        &mut self,
        addr: usize,
        fd: ProcessFileDescriptor,
        length: usize,
        offset: i64,
        writeable: bool,
    ) -> Result<bool> {
        let offset = u64::try_from(offset).map_err(|_| Error::BadOffset)?;
        let (fs, inode) = self.inode_of(fd)?;
        let offset_in_pages: u32 = (offset / PAGE_FRAME_SIZE as u64)
            .try_into()
            .map_err(|_| Error::BadOffset)?;
        self.mmap_inode(addr, fs, inode, length, offset_in_pages, writeable)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::user_program::syscall;
    use crate::vfs::tempfs::TempFS;
    use std::ffi::CStr;
    fn test_pcb(root: &RootFileSystem) -> ProcessControlBlock {
        ProcessControlBlock {
            pid: 0,
            ppid: 0,
            child_tids: vec![],
            waiting_thread: None,
            exit_code: None,
            vmas: Default::default(),
            cwd: root.get_root().unwrap(),
            cwd_path: "/".into(),
        }
    }
    // open file for fake PID of 0 with cwd / for testing
    fn open(root: &mut RootFileSystem, path: &Path, mode: Mode) -> Result<ProcessFileDescriptor> {
        let pid = 0;
        let fd = root.open(&test_pcb(root), path, mode)?;
        Ok(ProcessFileDescriptor { fd, pid })
    }
    // create file with the given contents
    fn create(
        root: &mut RootFileSystem,
        name: &str,
        mut contents: &[u8],
    ) -> Result<ProcessFileDescriptor> {
        let pcb = test_pcb(root);
        let fd = root.open(&pcb, name, Mode::CreateReadWrite)?;
        let fd = ProcessFileDescriptor { fd, pid: pcb.pid };
        while !contents.is_empty() {
            let n = root.write(fd, contents)?;
            assert!(n > 0);
            contents = &contents[n..];
        }
        Ok(fd)
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
        let pcb = test_pcb(&root);
        let fs2 = TempFS::new();
        root.mkdir(&pcb, "/2").unwrap();
        root.mount(&pcb, "/2", fs2).unwrap();
        let fs3 = TempFS::new();
        root.mkdir(&pcb, "/2/3").unwrap();
        root.mount(&pcb, "/2/3", fs3).unwrap();
        for path in ["/foo", "/2/foo", "/2/3/foo"] {
            let file = open(&mut root, path, Mode::CreateReadWrite).unwrap();
            // we shouldn't be allowed to unmount the FS file is contained in while it's open
            assert!(matches!(
                root.unmount(&pcb, dirname_and_filename(path).0),
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
            root.unmount(&pcb, "/2"),
            Err(Error::FileSystemInUse)
        ));
        root.unmount(&pcb, "/2/3").unwrap();
        root.unmount(&pcb, "/2").unwrap();
    }
    #[test]
    fn unlink() {
        let mut root = RootFileSystem::new();
        let fs = TempFS::new();
        root.mount_root(fs).unwrap();
        let pcb = test_pcb(&root);
        let fd = open(&mut root, "/file", Mode::CreateReadWrite).unwrap();
        root.unlink(&pcb, "/file").unwrap();
        // should still be able to write to file...
        root.write(fd, b"hello").unwrap();
        // but not open it
        assert!(matches!(
            root.open(&pcb, "/file", Mode::ReadWrite).unwrap_err(),
            Error::NotFound
        ));
        root.close(fd).unwrap();
        assert!(matches!(root.close(fd).unwrap_err(), Error::BadFd));
    }
    #[test]
    fn link_symlink() {
        let mut root = RootFileSystem::new();
        let fs = TempFS::new();
        root.mount_root(fs).unwrap();
        let pcb = test_pcb(&root);
        let fd = create(&mut root, "/file", b"hello").unwrap();
        for method in [RootFileSystem::link, RootFileSystem::symlink] {
            method(&mut root, &pcb, "/file", "/file2").unwrap();
            let fd2 = open(&mut root, "/file2", Mode::ReadWrite).unwrap();
            let info1 = root.fstat(fd).unwrap();
            let info2 = root.fstat(fd2).unwrap();
            assert_eq!(info1.inode, info2.inode);
            assert_eq!(info1.size, info2.size);
            assert_eq!(info1.nlink, info2.nlink);
            assert_eq!(info1.size, 5);
            root.unlink(&pcb, "/file2").unwrap();
            root.close(fd2).unwrap();
        }
        root.close(fd).unwrap();
        root.mkdir(&pcb, "/mount").unwrap();
        root.mount(&pcb, "/mount", TempFS::new()).unwrap();
        // make sure we can't hardlink across filesystems
        assert!(matches!(
            root.link(&pcb, "/file", "/mount/file").unwrap_err(),
            Error::HardLinkBetweenFileSystems
        ));
        // …but we can symlink across filesystems
        root.symlink(&pcb, "/file", "/mount/file").unwrap();
    }
    #[test]
    fn rename() {
        let mut root = RootFileSystem::new();
        let fs = TempFS::new();
        root.mount_root(fs).unwrap();
        let pcb = test_pcb(&root);
        let fd = create(&mut root, "/file", b"hello").unwrap();
        root.rename(&pcb, "/file", "/file2").unwrap();
        root.close(fd).unwrap();
        let fd = open(&mut root, "/file2", Mode::ReadWrite).unwrap();
        let mut buf = [0; 6];
        assert_eq!(root.read(fd, &mut buf).unwrap(), 5);
        assert_eq!(&buf, b"hello\0");
    }
    #[test]
    fn dirents() {
        let mut root = RootFileSystem::new();
        let fs = TempFS::new();
        root.mount_root(fs).unwrap();
        let pcb = test_pcb(&root);
        let fd = create(&mut root, "/file", b"test").unwrap();
        root.close(fd).unwrap();
        let fd = create(&mut root, "/file2", b"test").unwrap();
        root.close(fd).unwrap();
        root.mkdir(&pcb, "/dir").unwrap();
        assert!(std::mem::align_of::<u64>() >= std::mem::align_of::<Dirent>());
        let mut dirents = vec![0u64; 1024];
        let dir = open(&mut root, "/", Mode::ReadWrite).unwrap();
        let n = unsafe {
            root.getdents(
                dir,
                dirents.as_mut_ptr().cast(),
                dirents.len() * std::mem::size_of_val(&dirents[0]),
            )
        }
        .unwrap();
        let mut offset = 0;
        let mut entries = vec![];
        let dirents_ptr: *const u8 = dirents.as_ptr().cast();
        while offset < n {
            let dirent_ptr: *const Dirent = unsafe { dirents_ptr.add(offset).cast() };
            assert!(dirent_ptr.is_aligned());
            let dirent: &Dirent = unsafe { &*dirent_ptr };
            let name_offset = std::mem::offset_of!(Dirent, name);
            let name_ptr = unsafe { dirent_ptr.cast::<std::ffi::c_char>().add(name_offset) };
            let name: &str = unsafe { CStr::from_ptr(name_ptr) }.to_str().unwrap();
            entries.push((name.to_owned(), *dirent));
            offset += usize::from(dirent.reclen);
        }
        // seek back to entries[2] to test that lseek works correctly for directories
        root.lseek(dir, SeekFrom::Start, entries[2].1.offset)
            .unwrap();
        let n = unsafe {
            root.getdents(
                dir,
                dirents.as_mut_ptr().cast(),
                dirents.len() * std::mem::size_of_val(&dirents[0]),
            )
        }
        .unwrap();
        let dirent: Dirent = unsafe { dirents.as_ptr().cast::<Dirent>().read() };
        assert_eq!(dirent.inode, entries[2].1.inode);
        assert_eq!(usize::from(dirent.reclen), n);
        // now sort the directory entries, and make sure they are correct
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(entries[0].0, "dir");
        assert_eq!(entries[1].0, "file");
        assert_eq!(entries[2].0, "file2");
        assert_eq!(entries[0].1.r#type, syscall::S_DIRECTORY);
        assert_eq!(entries[1].1.r#type, syscall::S_REGULAR_FILE);
        assert_eq!(entries[2].1.r#type, syscall::S_REGULAR_FILE);
    }
    #[test]
    fn ftruncate() {
        let mut root = RootFileSystem::new();
        let fs = TempFS::new();
        root.mount_root(fs).unwrap();
        let fd = create(&mut root, "/file", b"test").unwrap();
        root.ftruncate(fd, 10).unwrap();
        let stat = root.fstat(fd).unwrap();
        assert_eq!(stat.size, 10);
        root.lseek(fd, SeekFrom::Start, 0).unwrap();
        let mut buf = [0; 10];
        root.read(fd, &mut buf).unwrap();
        assert_eq!(&buf, b"test\0\0\0\0\0\0");
        root.close(fd).unwrap();
    }
}
