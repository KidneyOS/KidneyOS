pub mod tempfs;

use alloc::borrow::ToOwned;
use alloc::string::String;

pub type INodeNum = u32;
pub type Path = str;
pub type OwnedPath = String;

/// Represents an open file
///
/// **IMPORTANT**: the kernel must call [`FileSystem::release`]
/// when it closes its last open file to an inode. Otherwise,
/// the filesystem will have to keep around the file's data indefinitely!
#[derive(Debug, Clone, Copy)]
pub struct FileHandle {
    /// inode number of this file
    pub inode: INodeNum,
    /// allows filesystem to store its own metadata about open files
    pub fs_data: usize,
}

#[derive(Debug, Clone, Copy)]
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
    /// Error accessing underlying storage device
    IO,
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
            Self::IO => write!(f, "I/O error"),
        }
    }
}

impl core::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;

/// File or directory information, as returned by stat.
pub struct FileInfo {
    /// Whether this is a file, directory, etc.
    pub r#type: INodeType,
    /// inode number
    pub inode: INodeNum,
    /// Size in bytes
    pub size: u64,
    /// Number of hard links
    pub nlink: u16,
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

/// Directory entry information
pub struct DirEntry<'a> {
    /// Type of entry
    pub r#type: INodeType,
    /// inode number
    pub inode: INodeNum,
    /// Name of entry
    pub name: &'a Path,
}

/// Directory entry which owns its path, for convenience.
pub struct OwnedDirEntry {
    /// Type of entry
    pub r#type: INodeType,
    /// inode number
    pub inode: INodeNum,
    /// Name of entry
    pub name: OwnedPath,
}

impl DirEntry<'_> {
    pub fn to_owned(&self) -> OwnedDirEntry {
        OwnedDirEntry {
            r#type: self.r#type,
            inode: self.inode,
            name: self.name.to_owned(),
        }
    }
}

pub trait DirectoryIterator: Sized {
    /// Get next file.
    ///
    /// Returns `Ok(None)` if the end of the directory was reached.
    fn next(&mut self) -> Result<Option<DirEntry<'_>>>;
    /// Get current offset into directory.
    ///
    /// The kernel should treat this as an opaque value that can be used for subsequent calls
    /// to [`FileSystem::readdir`] — e.g. it doesn't necessarily represent the number of directory
    /// entries that have been iterated.
    fn offset(&self) -> u64;
}

pub trait FileSystem {
    /// Get root inode number
    fn root(&self) -> INodeNum;
    /// Look up file in directory
    fn lookup(&self, parent: FileHandle, name: &Path) -> Result<INodeNum>;
    /// Open an existing file/directory/symlink.
    ///
    /// If the inode doesn't exist, returns [`Error::NotFound`].
    fn open(&mut self, inode: INodeNum) -> Result<FileHandle>;
    /// Create a new file in parent, or open it if it already exists (without truncating).
    ///
    /// The kernel must ensure that `parent` is a directory and that `name` is non-empty and doesn't contain `/`
    fn create(&mut self, parent: FileHandle, name: &Path) -> Result<FileHandle>;
    /// Make directory in parent
    ///
    /// The kernel must ensure that `parent` is a directory and that `name` is non-empty and doesn't contain `/`
    /// If `name` already exists (whether as a directory or as a file), returns [`Error::Exists`].
    fn mkdir(&mut self, parent: FileHandle, name: &Path) -> Result<()>;
    /// Remove a (link to a) file/symlink in parent
    ///
    /// The kernel must ensure that `parent` is a directory and that `name` is non-empty and doesn't contain `/`
    /// The filesystem must keep around the file data in memory or on disk until [`Self::release`] is called.
    fn unlink(&mut self, parent: FileHandle, name: &Path) -> Result<()>;
    /// Remove a directory in parent
    ///
    /// The kernel must ensure that `parent` is a directory before calling this
    /// The filesystem must not free the inode corresponding to this directory until [`Self::release`] is called.
    fn rmdir(&mut self, parent: FileHandle, name: &Path) -> Result<()>;
    /// Read entries in a directory
    ///
    /// The kernel must ensure that `dir` is a directory before calling this.
    ///
    /// `offset` must either be zero or a value previously returned by [`DirectoryIterator::offset`].
    /// If entries are added/removed from the directory in between the call to [`DirectoryIterator::offset`]
    /// and this call, they may or may not be listed. But modifications to the directory must not
    /// cause directory entries to be repeated or unmodified entries to be skipped.
    fn readdir(&self, dir: FileHandle, offset: u64) -> impl '_ + DirectoryIterator;
    /// Indicate that there are no more references to an inode
    /// (i.e. all file descriptors pointing to it have been closed).
    ///
    /// If there are no links left to the file, the filesystem should delete it at this point.
    /// The kernel must not use any file handle pointing to this inode after calling this.
    fn release(&mut self, inode: INodeNum);
    /// Read from file into buf at offset.
    ///
    /// The kernel must ensure that `file` is a regular file before calling this.
    fn read(&self, file: FileHandle, offset: u64, buf: &mut [u8]) -> Result<usize>;
    /// Write to file from buf at offset.
    ///
    /// The kernel must ensure that `file` is a regular file before calling this.
    fn write(&mut self, file: FileHandle, offset: u64, buf: &[u8]) -> Result<usize>;
    /// Get information about an open file/symlink/directory.
    fn stat(&self, file: FileHandle) -> Result<FileInfo>;
    /// Create a hard link
    ///
    /// As on Linux, this returns [`Error::Exists`] and does nothing if the destination already exists.
    ///
    /// The kernel must ensure that parent is a directory, and that `name` is non-empty and doesn't contain `/`
    fn link(&mut self, source: FileHandle, parent: FileHandle, name: &Path) -> Result<()>;
    /// Create a symbolic link
    ///
    /// As on Linux, this returns [`Error::Exists`] and does nothing if the destination already exists.
    ///
    /// The kernel must ensure that parent is a directory, and that `link` and `name` are non-empty and that `name` doesn't contain `/`
    fn symlink(&mut self, link: &Path, parent: FileHandle, name: &Path) -> Result<()>;
    /// Read a symbolic link
    ///
    /// Returns the prefix of `buf` which has been filled with the desintation, or `Ok(None)` if `buf`
    /// is too short (in which case the contents of `buf` are unspecified).
    ///
    /// Note that you can get the size of buffer needed from calling [`FileInfo::size`]
    /// on the return value of [`Self::stat`].
    fn readlink<'a>(&self, link: FileHandle, buf: &'a mut Path) -> Result<Option<&'a str>>;
    /// Set a new file size.
    ///
    /// If this is less than the previous size, the extra data is lost.
    /// If it's larger than the previous size, the extended part should be filled with
    /// null bytes.
    ///
    /// The kernel must ensure that `file` is a regular file before calling this.
    fn truncate(&mut self, file: FileHandle, size: u64) -> Result<()>;
}
