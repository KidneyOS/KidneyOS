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
pub trait FileHandle: core::fmt::Debug {
    fn inode(self) -> INodeNum;
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
            Self::IO(s) => write!(f, "I/O error: {s}"),
        }
    }
}

impl core::error::Error for Error {}

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

#[derive(Debug)]
pub struct DirEntries {
    /// Raw directory entries, with names pointing to [`Self::filenames`]
    pub entries: Vec<RawDirEntry>,
    /// Null ('\0') separated string of all file names in this directory.
    pub filenames: String,
}

impl DirEntries {
    pub fn get_filename(&self, name: usize) -> &str {
        let s = &self.filenames[name..];
        &s[..s.find('\0').unwrap_or(s.len())]
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

pub trait FileSystem {
    type FileHandle: FileHandle;
    /// Get root inode number
    fn root(&self) -> INodeNum;
    /// Open an existing file/directory/symlink.
    ///
    /// If the inode doesn't exist, returns [`Error::NotFound`].
    fn open(&mut self, inode: INodeNum) -> Result<Self::FileHandle>;
    /// Create a new file in parent, or open it if it already exists (without truncating).
    ///
    /// The kernel must ensure that `parent` is a directory and that `name` is non-empty and doesn't contain `/`
    fn create(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<Self::FileHandle>;
    /// Make directory in parent
    ///
    /// The kernel must ensure that `parent` is a directory and that `name` is non-empty and doesn't contain `/`
    /// If `name` already exists (whether as a directory or as a file), returns [`Error::Exists`].
    fn mkdir(&mut self, parent: &mut Self::FileHandle, name: &Path) -> Result<()>;
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
    fn symlink(&mut self, link: &Path, parent: &mut Self::FileHandle, name: &Path) -> Result<()>;
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
        buf: &'a mut Path,
    ) -> Result<Option<&'a str>>;
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
