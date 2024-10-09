#[cfg(test)]
pub mod read_only_test;
pub mod tempfs;

use alloc::{borrow::Cow, format, string::String, vec::Vec};

pub type INodeNum = u32;
pub type Path = str;
pub type OwnedPath = String;

/// Represents an open file
///
/// **IMPORTANT**: the kernel must call [`FileSystem::release`]
/// when it closes its last open file to an inode. Otherwise,
/// the filesystem will have to keep around the file's data indefinitely!
pub trait FileHandle: core::fmt::Debug + Send + Sync {
    fn inode(&self) -> INodeNum;
}

#[derive(Debug, Clone)]
pub enum Error {
    /// directory entry not found
    NotFound,
    /// operation expecting directory called with something that isn't a directory
    NotDirectory,
    /// operation expecting file called with a directory
    IsDirectory,
    /// no space left on device
    NoSpace,
    /// Too many hard links to file
    TooManyLinks,
    /// Called rmdir on non-empty directory
    NotEmpty,
    /// Target destination already exists
    Exists,
    /// Unsupported operation (e.g. file system does not support symlinks)
    Unsupported,
    /// Write operation to a read-only file system
    ReadOnlyFS,
    /// Process has too many open file descriptors
    TooManyOpenFiles,
    /// Bad file descriptor
    BadFd,
    /// Trying to unmount file system with open files
    FileSystemInUse,
    /// Seek to bad offset
    BadOffset,
    /// Seek in non-seekable file
    IllegalSeek,
    /// Unmount directory that isn't mounted
    NotMounted,
    /// Called readlink on something that isn't a link
    NotLink,
    /// Too many levels of symbolic links
    TooManyLevelsOfLinks,
    /// Error accessing underlying storage device
    IO(String),
}

impl From<crate::block::block_error::BlockError> for Error {
    fn from(value: crate::block::block_error::BlockError) -> Self {
        Self::IO(format!("{value}"))
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound => write!(f, "not found"),
            Self::NotDirectory => write!(f, "not a directory"),
            Self::IsDirectory => write!(f, "is a directory"),
            Self::NoSpace => write!(f, "no space left on device"),
            Self::TooManyLinks => write!(f, "too many hard links to file"),
            Self::NotEmpty => write!(f, "directory not empty"),
            Self::Exists => write!(f, "destination already exists"),
            Self::Unsupported => write!(f, "unsupported operation"),
            Self::ReadOnlyFS => write!(f, "read-only file system"),
            Self::TooManyOpenFiles => write!(f, "too many open files"),
            Self::BadFd => write!(f, "bad file descriptor"),
            Self::FileSystemInUse => write!(f, "file system in use"),
            Self::BadOffset => write!(f, "seek to bad offset"),
            Self::IllegalSeek => write!(f, "illegal seek"),
            Self::NotMounted => write!(f, "not mounted"),
            Self::NotLink => write!(f, "not a link"),
            Self::TooManyLevelsOfLinks => write!(f, "too many levels of symbolic links"),
            Self::IO(s) => write!(f, "I/O error: {s}"),
        }
    }
}

impl core::error::Error for Error {}

impl Error {
    pub fn to_isize(&self) -> isize {
        use crate::user_program::syscall;
        match self {
            Error::NotFound => syscall::ENOENT,
            Error::NotDirectory => syscall::ENOTDIR,
            Error::IsDirectory => syscall::EISDIR,
            Error::NoSpace => syscall::ENOSPC,
            Error::TooManyLinks => syscall::EMLINK,
            Error::NotEmpty => syscall::ENOTEMPTY,
            Error::Exists => syscall::EEXIST,
            Error::Unsupported => syscall::EIO,
            Error::ReadOnlyFS => syscall::EROFS,
            Error::TooManyOpenFiles => syscall::EMFILE,
            Error::BadFd => syscall::EBADF,
            Error::FileSystemInUse => syscall::EBUSY,
            Error::BadOffset => syscall::EINVAL,
            Error::IllegalSeek => syscall::ESPIPE,
            Error::NotMounted => syscall::EINVAL,
            Error::NotLink => syscall::EINVAL,
            Error::TooManyLevelsOfLinks => syscall::ELOOP,
            Error::IO(_) => syscall::EIO,
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

/// File or directory information, as returned by stat.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Whether this is a file, directory, etc.
    pub r#type: INodeType,
    /// inode number
    pub inode: INodeNum,
    /// Size in bytes
    pub size: u64,
    /// Number of hard links
    pub nlink: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum INodeType {
    /// Regular file
    File,
    /// Symbolic link
    Link,
    /// Directory
    Directory,
}

/// Raw directory entry information
///
/// Rather than containing a reference to a string or an owned string for the file name,
/// this contains an offset into [`DirEntries::filenames`]
#[derive(Debug, Clone, Copy)]
pub struct RawDirEntry {
    /// Type of entry
    pub r#type: INodeType,
    /// inode number
    pub inode: INodeNum,
    /// Name of entry - this should be an index into [`DirEntries::filenames`]
    pub name: usize,
}

/// Directory entry information
#[derive(Debug, Clone)]
pub struct DirEntry<'a> {
    /// Type of entry
    pub r#type: INodeType,
    /// inode number
    pub inode: INodeNum,
    /// Name of entry
    pub name: Cow<'a, str>,
}

/// A directory entry which owns its path
pub type OwnedDirEntry = DirEntry<'static>;

impl DirEntry<'_> {
    pub fn to_owned(&self) -> OwnedDirEntry {
        OwnedDirEntry {
            r#type: self.r#type,
            inode: self.inode,
            name: Cow::Owned(String::from(self.name.as_ref())),
        }
    }
}

pub struct DirIterator<'a> {
    entries: &'a DirEntries,
    it: alloc::slice::Iter<'a, RawDirEntry>,
}

impl<'a> Iterator for DirIterator<'a> {
    type Item = DirEntry<'a>;
    fn next(&mut self) -> Option<DirEntry<'a>> {
        let raw = self.it.next()?;
        Some(DirEntry {
            inode: raw.inode,
            r#type: raw.r#type,
            name: Cow::Borrowed(self.entries.get_filename(raw.name)),
        })
    }
}

#[derive(Debug, Default)]
pub struct DirEntries {
    /// Raw directory entries, with names pointing to [`Self::filenames`]
    pub entries: Vec<RawDirEntry>,
    /// Null ('\0') separated string of all file names in this directory.
    pub filenames: String,
}

impl DirEntries {
    /// Create a new empty list of directory entries.
    pub fn new() -> Self {
        Self::default()
    }
    /// Get filename associated with name ID, from [`RawDirEntry::name`].
    pub fn get_filename(&self, name: usize) -> &str {
        let s = &self.filenames[name..];
        &s[..s.find('\0').unwrap_or(s.len())]
    }
    pub fn add(&mut self, inode: INodeNum, r#type: INodeType, name: &str) {
        let name_id = self.filenames.len();
        self.filenames.push_str(name);
        self.filenames.push('\0');
        self.entries.push(RawDirEntry {
            inode,
            r#type,
            name: name_id,
        });
    }
    #[cfg(test)]
    /// Collect directory entries into a Vec, sorted by name.
    ///
    /// Useful for testing, but probably shouldn't be used in the kernel,
    /// because it has a separate allocation for each file name.
    pub fn to_sorted_vec(&self) -> Vec<OwnedDirEntry> {
        let mut entries: Vec<OwnedDirEntry> = self.into_iter().map(|e| e.to_owned()).collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }
}

impl<'a> IntoIterator for &'a DirEntries {
    type IntoIter = DirIterator<'a>;
    type Item = DirEntry<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DirIterator {
            entries: self,
            it: self.entries.iter(),
        }
    }
}

pub trait FileSystem: Sized + Sync + Send {
    type FileHandle: FileHandle;
    /// Get root inode number
    fn root(&self) -> INodeNum;
    /// Open an existing file/directory/symlink.
    ///
    /// If the inode doesn't exist (e.g. it was deleted between the call to [`FileSystem::readdir`]
    /// which discovered it and now), returns [`Error::NotFound`].
    fn open(&mut self, inode: INodeNum) -> Result<Self::FileHandle>;
    /// Create a new file in parent, or open it if it already exists (without truncating).
    ///
    /// The kernel must ensure that `parent` is a directory and that `name` is non-empty and doesn't contain `/`
    fn create(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<Self::FileHandle>;
    /// Make directory in parent
    ///
    /// The kernel must ensure that `parent` is a directory and that `name` is non-empty and doesn't contain `/`
    /// If `name` already exists (whether as a directory or as a file), returns [`Error::Exists`].
    fn mkdir(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<INodeNum>;
    /// Remove a (link to a) file/symlink in parent
    ///
    /// The kernel must ensure that `parent` is a directory and that `name` is non-empty and doesn't contain `/`
    /// The filesystem must keep around the file data in memory or on disk until [`Self::release`] is called.
    fn unlink(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<()>;
    /// Remove a directory in parent
    ///
    /// The kernel must ensure that `parent` is a directory before calling this
    /// The filesystem must not free the inode corresponding to this directory until [`Self::release`] is called.
    fn rmdir(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<()>;
    /// Read all entries in a directory
    ///
    /// The kernel must ensure that `dir` is a directory before calling this.
    fn readdir(&mut self, dir: &mut Self::FileHandle) -> Result<DirEntries>;
    /// Indicate that there are no more references to an inode
    /// (i.e. all file descriptors pointing to it have been closed).
    ///
    /// If there are no links left to the file, the filesystem should delete it at this point.
    /// The kernel must not use any file handle pointing to this inode after calling this
    /// (without first calling `open` again).
    fn release(&mut self, inode: INodeNum);
    /// Read from file into buf at offset.
    ///
    /// The kernel must ensure that `file` is a regular file before calling this.
    fn read(&mut self, file: &mut Self::FileHandle, offset: u64, buf: &mut [u8]) -> Result<usize>;
    /// Write to file from buf at offset.
    ///
    /// The kernel must ensure that `file` is a regular file before calling this.
    fn write(&mut self, file: &mut Self::FileHandle, offset: u64, buf: &[u8]) -> Result<usize>;
    /// Get information about an open file/symlink/directory.
    fn stat(&mut self, file: &Self::FileHandle) -> Result<FileInfo>;
    /// Create a hard link
    ///
    /// As on Linux, this returns [`Error::Exists`] and does nothing if the destination already exists.
    ///
    /// The kernel must ensure that parent is a directory, and that `name` is non-empty and doesn't contain `/`
    fn link(
        &mut self,
        source: &mut Self::FileHandle,
        parent: &mut Self::FileHandle,
        name: &Path,
    ) -> Result<()>;
    /// Create a symbolic link
    ///
    /// As on Linux, this returns [`Error::Exists`] and does nothing if the destination already exists.
    ///
    /// The kernel must ensure that parent is a directory, and that `link` and `name` are non-empty and that `name` doesn't contain `/`
    fn symlink(
        &mut self,
        link: &Path,
        parent: &mut Self::FileHandle,
        name: &Path,
    ) -> Result<INodeNum>;
    /// Read a symbolic link
    ///
    /// Returns the prefix of `buf` which has been filled with the desintation, or `Ok(None)` if `buf`
    /// is too short (in which case the contents of `buf` are unspecified).
    ///
    /// Note that you can get the size of buffer needed by accessing [`FileInfo::size`]
    /// on the return value of [`Self::stat`].
    fn readlink<'a>(
        &mut self,
        link: &mut Self::FileHandle,
        buf: &'a mut [u8],
    ) -> Result<Option<&'a Path>>;
    /// Set a new file size.
    ///
    /// If this is less than the previous size, the extra data is lost.
    /// If it's larger than the previous size, the extended part should be filled with
    /// null bytes.
    ///
    /// The kernel must ensure that `file` is a regular file before calling this.
    fn truncate(&mut self, file: &mut Self::FileHandle, size: u64) -> Result<()>;
    /// Sync changes to disk.
    ///
    /// Blocks until all previous operations have been committed to disk.
    /// All other functions can just perform operations on cached copies of data
    /// in memory; this is the only way of ensuring that the data is actually saved.
    fn sync(&mut self) -> Result<()>;
}

/// File system that doesn't have any extra state to keep track of for open files.
///
/// This trait also has default stub implementations for all the filesystem functions except for [`FileSystem::root`],
/// so you can implement and test them one at a time
#[allow(unused_variables)] // default implementations don't always use their parameters
pub trait SimpleFileSystem: Sized + Send + Sync {
    /// Get root inode number.
    fn root(&self) -> INodeNum;
    /// The kernel will always call this function before reading/writing data to a file.
    ///
    /// This should return [`Error::NotFound`] if `inode` doesn't exist.
    /// You should keep track of which open files haven't been released yet (via [`SimpleFileSystem::release`])
    /// so that you can keep them around even when they are unlinked.
    fn open(&mut self, inode: INodeNum) -> Result<()> {
        Ok(())
    }
    /// Create an empty file in `parent` called `name`, returning the inode number of the file.
    fn create(&mut self, parent: INodeNum, name: &Path) -> Result<INodeNum> {
        Err(Error::Unsupported)
    }
    /// Create an empty directory in `parent` called `name`.
    ///
    /// Returns the inode number of the newly-created directory
    fn mkdir(&mut self, parent: INodeNum, name: &Path) -> Result<INodeNum> {
        Err(Error::Unsupported)
    }
    /// Unlink the file called `name` in the directory `parent`.
    fn unlink(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    /// Remove the directory in `parent` called `name`.
    fn rmdir(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    /// Read the entire contents of a directory.
    ///
    /// A [`DirEntries`] object can be constructed with the [`DirEntries::new`] and [`DirEntries::add`] functions.
    fn readdir(&mut self, dir: INodeNum) -> Result<DirEntries> {
        Err(Error::Unsupported)
    }
    /// Release an inode, indicating that there are no open handles to it.
    ///
    /// If (and only if!) there are no links left to the file, the file system should delete it.
    fn release(&mut self, inode: INodeNum) {}
    /// Read from a file at offset `offset`, into `buf`.
    ///
    /// Returns the number of bytes successfully read.
    fn read(&mut self, file: INodeNum, offset: u64, buf: &mut [u8]) -> Result<usize> {
        Err(Error::Unsupported)
    }
    /// Write to a file at offset `offset`, from `buf`.
    ///
    /// Returns the number of bytes successfully written.
    fn write(&mut self, file: INodeNum, offset: u64, buf: &[u8]) -> Result<usize> {
        Err(Error::Unsupported)
    }
    /// Get information about `file`.
    fn stat(&mut self, file: INodeNum) -> Result<FileInfo> {
        Err(Error::Unsupported)
    }
    /// Create hard link to `source` in `parent` called `name`.
    fn link(&mut self, source: INodeNum, parent: INodeNum, name: &Path) -> Result<()> {
        Err(Error::Unsupported)
    }
    /// Create symbolic link to `link` in `parent` called `name`.
    ///
    /// Returns the inode number of the newly-created symbolic link
    fn symlink(&mut self, link: &Path, parent: INodeNum, name: &Path) -> Result<INodeNum> {
        Err(Error::Unsupported)
    }
    /// Read the contents of a symbolic link
    fn readlink(&mut self, link: INodeNum) -> Result<String> {
        Err(Error::Unsupported)
    }
    /// Version of [`SimpleFileSystem::readlink`] that doesn't allocate.
    ///
    /// If you implement [`SimpleFileSystem::readlink`], this will be provided automatically.
    fn readlink_no_alloc<'a>(
        &mut self,
        link: INodeNum,
        buf: &'a mut [u8],
    ) -> Result<Option<&'a str>> {
        let contents = self.readlink(link)?;
        if buf.len() < contents.len() {
            return Ok(None);
        }
        let buf = &mut buf[..contents.len()];
        buf.copy_from_slice(contents.as_bytes());
        Ok(Some(core::str::from_utf8(buf).expect(
            "should be UTF-8 since it was copied from a string",
        )))
    }
    /// Set size of `file` to `size`.
    ///
    /// If this increases the size of the file, the extra space should be filled with zeroes.
    fn truncate(&mut self, file: INodeNum, size: u64) -> Result<()> {
        Err(Error::Unsupported)
    }
    /// Sync changes to disk.
    fn sync(&mut self) -> Result<()> {
        Ok(())
    }
}

/// File handle for [`SimpleFileSystem`] file systems.
#[derive(Clone, Copy)]
pub struct SimpleFileHandle(INodeNum);
impl core::fmt::Debug for SimpleFileHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "FileHandle({})", self.0)
    }
}

impl FileHandle for SimpleFileHandle {
    fn inode(&self) -> INodeNum {
        self.0
    }
}

impl<F: SimpleFileSystem> FileSystem for F {
    type FileHandle = SimpleFileHandle;
    fn root(&self) -> INodeNum {
        SimpleFileSystem::root(self)
    }
    fn open(&mut self, inode: INodeNum) -> Result<Self::FileHandle> {
        SimpleFileSystem::open(self, inode)?;
        Ok(SimpleFileHandle(inode))
    }
    fn create(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<Self::FileHandle> {
        SimpleFileSystem::create(self, parent.0, name).map(SimpleFileHandle)
    }
    fn mkdir(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<INodeNum> {
        SimpleFileSystem::mkdir(self, parent.0, name)
    }
    fn unlink(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<()> {
        SimpleFileSystem::unlink(self, parent.0, name)
    }
    fn rmdir(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<()> {
        SimpleFileSystem::rmdir(self, parent.0, name)
    }
    fn readdir(&mut self, dir: &mut Self::FileHandle) -> Result<DirEntries> {
        SimpleFileSystem::readdir(self, dir.0)
    }
    fn release(&mut self, inode: INodeNum) {
        SimpleFileSystem::release(self, inode)
    }
    fn read(&mut self, file: &mut Self::FileHandle, offset: u64, buf: &mut [u8]) -> Result<usize> {
        SimpleFileSystem::read(self, file.0, offset, buf)
    }
    fn write(&mut self, file: &mut Self::FileHandle, offset: u64, buf: &[u8]) -> Result<usize> {
        SimpleFileSystem::write(self, file.0, offset, buf)
    }
    fn stat(&mut self, file: &Self::FileHandle) -> Result<FileInfo> {
        SimpleFileSystem::stat(self, file.0)
    }
    fn link(
        &mut self,
        source: &mut Self::FileHandle,
        parent: &mut Self::FileHandle,
        name: &Path,
    ) -> Result<()> {
        SimpleFileSystem::link(self, source.0, parent.0, name)
    }
    fn symlink(
        &mut self,
        link: &Path,
        parent: &mut Self::FileHandle,
        name: &Path,
    ) -> Result<INodeNum> {
        SimpleFileSystem::symlink(self, link, parent.0, name)
    }
    fn readlink<'a>(
        &mut self,
        link: &mut Self::FileHandle,
        buf: &'a mut [u8],
    ) -> Result<Option<&'a Path>> {
        SimpleFileSystem::readlink_no_alloc(self, link.0, buf)
    }
    fn truncate(&mut self, file: &mut Self::FileHandle, size: u64) -> Result<()> {
        SimpleFileSystem::truncate(self, file.0, size)
    }
    fn sync(&mut self) -> Result<()> {
        SimpleFileSystem::sync(self)
    }
}
