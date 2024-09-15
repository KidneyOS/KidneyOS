#[cfg(not(test))]
use crate::println;
#[cfg(test)]
use std::println;

use crate::vfs::{
    DirEntry, DirectoryIterator, Error, FileHandle, FileInfo, FileSystem, INodeNum, INodeType,
    Path, Result,
};
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::{cmp::min, mem::size_of, num::NonZeroUsize};

struct TempFile {
    nlink: u16,
    data: Vec<u8>,
}

struct TempDirectory {
    entries: BTreeMap<String, INodeNum>,
    nlink: u16,
}

struct TempLink {
    path: String,
    nlink: u16,
}

enum TempINode {
    File(TempFile),
    Directory(TempDirectory),
    Link(TempLink),
}

/// in-memory filesystem
pub struct TempFs {
    inodes: BTreeMap<INodeNum, TempINode>,
}

const ROOT_INO: INodeNum = 1;

pub struct TempDirectoryIterator<'a> {
    fs: &'a TempFs,
    it: alloc::collections::btree_map::Iter<'a, String, INodeNum>,
    filename: String,
}

impl<'a> DirectoryIterator<'a> for TempDirectoryIterator<'a> {
    fn next(&mut self) -> Result<Option<DirEntry>> {
        let Some((name, inode_num)) = self.it.next() else {
            return Ok(None);
        };
        let inode_num = *inode_num;
        let inode = self
            .fs
            .inodes
            .get(&inode_num)
            .expect("tempfs consistency error — reference to nonexistent inode");
        self.filename = name.into();
        let r#type = match inode {
            TempINode::File(_) => INodeType::File,
            TempINode::Directory(_) => INodeType::Directory,
            TempINode::Link(_) => INodeType::Link,
        };
        Ok(Some(DirEntry {
            inode: inode_num,
            r#type,
            name: &self.filename,
        }))
    }
}

impl Default for TempFs {
    fn default() -> Self {
        Self::new()
    }
}

const NO_INODE: &str = "Couldn't find inode — either kernel is using filesystem incorrectly or we freed an inode when we shouldn't have.";
impl TempFs {
    pub fn new() -> TempFs {
        let root = TempINode::Directory(TempDirectory {
            entries: BTreeMap::new(),
            nlink: 1,
        });
        let mut inodes = BTreeMap::new();
        inodes.insert(ROOT_INO, root);
        TempFs { inodes }
    }
    fn get_inode(&self, handle: FileHandle) -> &TempINode {
        self.inodes.get(&handle.inode).expect(NO_INODE)
    }
    fn get_inode_mut(&mut self, handle: FileHandle) -> &mut TempINode {
        self.inodes.get_mut(&handle.inode).expect(NO_INODE)
    }
    fn add_inode(&mut self, inode: TempINode) -> INodeNum {
        // Since inodes are stored in a BTreeMap, the last entry is the maximum inode value.
        // So we take one more than that. This isn't realistically going to overflow a u64.
        if size_of::<INodeNum>() < 8 {
            panic!(
                "this function should be updated to handle smaller inode size (u32 could overflow)"
            );
        }
        let inode_num = *self
            .inodes
            .last_entry()
            .expect("filesystem should at least contain root")
            .key()
            + 1;
        self.inodes.insert(inode_num, inode);
        inode_num
    }
    // performs either unlink or rmdir.
    fn unlink_or_rmdir(&mut self, parent: FileHandle, name: &Path, is_rmdir: bool) -> Result<()> {
        let parent_inode = self.get_inode(parent);
        let TempINode::Directory(parent_dir) = parent_inode else {
            panic!("Kernel should call stat to make sure this is a directory before removing something from it.");
        };
        let inode_num = *parent_dir.entries.get(name).ok_or(Error::NotFound)?;
        let inode = self
            .inodes
            .get_mut(&inode_num)
            .expect("inconsistent filesystem state — referenced inode doesn't exist");
        // Note that we don't actually remove the inode from inodes here;
        // we do that in `release`, so that existing file handles can still access
        // the file until then.
        match inode {
            TempINode::Directory(d) => {
                if !is_rmdir {
                    return Err(Error::NotDirectory);
                }
                if !d.entries.is_empty() {
                    return Err(Error::NotEmpty);
                }
                assert!(
                    d.nlink > 0,
                    "VFS rmdir called on an already-deleted directory"
                );
                d.nlink -= 1;
            }
            TempINode::File(f) => {
                if is_rmdir {
                    return Err(Error::NotDirectory);
                }
                assert!(f.nlink > 0, "VFS unlink called on file with 0 links");
                f.nlink -= 1;
            }
            TempINode::Link(l) => {
                if is_rmdir {
                    return Err(Error::NotDirectory);
                }
                assert!(
                    l.nlink > 0,
                    "VFS unlink called on an already-deleted symlink"
                );
                l.nlink -= 1;
            }
        }
        let parent_inode = self.get_inode_mut(parent);
        let TempINode::Directory(parent_dir) = parent_inode else {
            panic!("This should never happen due to check above.");
        };
        parent_dir.entries.remove(name);
        Ok(())
    }
}

const DEBUG_TEMPFS: bool = true;

impl FileSystem for TempFs {
    type DirectoryIterator<'a> = TempDirectoryIterator<'a>;
    fn root(&self) -> INodeNum {
        ROOT_INO
    }
    fn lookup(&self, parent: FileHandle, name: &Path) -> Result<INodeNum> {
        if DEBUG_TEMPFS {
            println!("tempfs: lookup in {}: {}", parent.inode, name);
        }
        let parent_inode = self.get_inode(parent);
        let TempINode::Directory(dir) = parent_inode else {
            return Err(Error::NotDirectory);
        };
        dir.entries.get(name).ok_or(Error::NotFound).copied()
    }
    fn open(&mut self, inode: INodeNum) -> Result<FileHandle> {
        if DEBUG_TEMPFS {
            println!("tempfs: open {}", inode);
        }
        if self.inodes.get(&inode).is_none() {
            return Err(Error::NotFound);
        }
        Ok(FileHandle { inode, fs_data: 0 })
    }
    fn create(&mut self, parent: FileHandle, name: &Path) -> Result<FileHandle> {
        if DEBUG_TEMPFS {
            println!("tempfs: create in {}: {}", parent.inode, name);
        }
        if name.is_empty() {
            panic!("Empty name passed to create");
        }
        // first check if file already exists
        let parent_inode = self.get_inode_mut(parent);
        let TempINode::Directory(parent_dir) = parent_inode else {
            panic!("Kernel should call stat to make sure this is a directory before creating a file in it.");
        };
        if parent_dir.nlink == 0 {
            // this directory has been rmdir'd
            return Err(Error::NotDirectory);
        }
        let inode_num = parent_dir.entries.get(name).copied().unwrap_or_else(|| {
            // create new file
            let inode = TempINode::File(TempFile {
                nlink: 1,
                data: Vec::new(),
            });
            let inode_num = self.add_inode(inode);
            let parent_inode = self.get_inode_mut(parent);
            let TempINode::Directory(parent_dir) = parent_inode else {
                panic!("should never happen due to check above");
            };
            parent_dir.entries.insert(name.into(), inode_num);
            inode_num
        });
        Ok(FileHandle {
            inode: inode_num,
            fs_data: 0,
        })
    }
    fn unlink(&mut self, parent: FileHandle, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: unlink in {}: {}", parent.inode, name);
        }
        if name.is_empty() {
            panic!("Empty name passed to unlink");
        }
        self.unlink_or_rmdir(parent, name, false)
    }
    fn rmdir(&mut self, parent: FileHandle, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: rmdir in {}: {}", parent.inode, name);
        }
        if name.is_empty() {
            panic!("Empty name passed to rmdir");
        }
        self.unlink_or_rmdir(parent, name, true)
    }
    fn readdir(&self, dir: FileHandle) -> TempDirectoryIterator<'_> {
        if DEBUG_TEMPFS {
            println!("tempfs: readdir {}", dir.inode);
        }
        let inode = self.get_inode(dir);
        let TempINode::Directory(dir) = inode else {
            panic!("Kernel should call stat to make sure this is a directory before calling readdir on it.");
        };
        TempDirectoryIterator {
            fs: self,
            it: dir.entries.iter(),
            filename: String::new(),
        }
    }
    fn release(&mut self, inode_num: INodeNum) {
        if DEBUG_TEMPFS {
            println!("tempfs: release {}", inode_num);
        }
        let inode = self
            .inodes
            .get(&inode_num)
            .expect("kernel should only call release on inodes it knows to exist");
        let should_delete = match inode {
            TempINode::Link(l) => l.nlink == 0,
            TempINode::Directory(d) => d.nlink == 0,
            TempINode::File(f) => f.nlink == 0,
        };
        if should_delete {
            // we can safely remove the inode.
            self.inodes.remove(&inode_num);
        }
    }
    fn read(&self, file: FileHandle, offset: u64, buf: &mut [u8]) -> Result<usize> {
        if DEBUG_TEMPFS {
            println!(
                "tempfs: read from {} @ offset {} length {}",
                file.inode,
                offset,
                buf.len()
            );
        }
        let inode = self.get_inode(file);
        let TempINode::File(f) = inode else {
            panic!("Kernel should make sure this is a regular file before reading from it.");
        };
        if offset >= f.data.len() as u64 {
            // can't read any data
            return Ok(0);
        }
        let offset = offset as usize; // fits into usize by check above
        let read_len = min(buf.len(), f.data.len() - offset);
        buf[..read_len].copy_from_slice(&f.data[offset..offset + read_len]);
        Ok(read_len)
    }
    fn write(&mut self, file: FileHandle, offset: u64, buf: &[u8]) -> Result<usize> {
        if DEBUG_TEMPFS {
            println!(
                "tempfs: write to {} @ offset {} length {}",
                file.inode,
                offset,
                buf.len()
            );
        }
        let inode = self.get_inode_mut(file);
        let TempINode::File(f) = inode else {
            panic!("Kernel should make sure this is a regular file before writing to it.");
        };
        if offset > (isize::MAX as u64).saturating_sub(buf.len() as u64) {
            // file data would exceed isize::MAX bytes
            return Err(Error::NoSpace);
        }
        let offset = offset as usize;
        // amount we need to grow the file by
        let grow_amount = (offset + buf.len()).saturating_sub(f.data.len());
        // return no space error if allocation failed
        f.data
            .try_reserve(grow_amount)
            .map_err(|_| Error::NoSpace)?;
        for _ in 0..grow_amount {
            // NOTE: files with holes will not perform well.
            f.data.push(0);
        }
        f.data[offset..offset + buf.len()].copy_from_slice(buf);
        Ok(buf.len())
    }
    fn stat(&self, file: FileHandle) -> Result<FileInfo> {
        if DEBUG_TEMPFS {
            println!("tempfs: stat {}", file.inode);
        }
        let inode = self.get_inode(file);
        match inode {
            TempINode::Directory(d) => Ok(FileInfo {
                r#type: INodeType::Directory,
                inode: file.inode,
                nlink: 1,
                // pretend that each entry takes up 1 byte (this doesn't matter much)
                size: d.entries.len() as u64,
            }),
            TempINode::File(f) => Ok(FileInfo {
                r#type: INodeType::File,
                inode: file.inode,
                nlink: f.nlink,
                size: f.data.len() as u64,
            }),
            TempINode::Link(l) => Ok(FileInfo {
                r#type: INodeType::Link,
                inode: file.inode,
                nlink: 1,
                size: l.path.len() as u64,
            }),
        }
    }
    fn link(&mut self, source: FileHandle, parent: FileHandle, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!(
                "tempfs: create link to {} in {}: {}",
                source.inode, parent.inode, name
            );
        }
        // check for existence
        let parent_inode = self.get_inode(parent);
        let TempINode::Directory(parent_dir) = parent_inode else {
            panic!("Kernel should make sure parent is a directory before creating a link in it.");
        };
        if parent_dir.nlink == 0 {
            // this directory has been rmdir'd
            return Err(Error::NotDirectory);
        }
        if parent_dir.entries.contains_key(name) {
            return Err(Error::Exists);
        }
        // increment link count
        let source_inode = self.get_inode_mut(source);
        let TempINode::File(f) = source_inode else {
            // currently don't support hard-linking symlinks/directories
            // (would be easy to fix)
            return Err(Error::TooManyLinks);
        };
        f.nlink = f.nlink.checked_add(1).ok_or(Error::TooManyLinks)?;
        // insert directory entry
        // we can't just reuse parent_inode from above, since we accessed self in between.
        let parent_inode = self.get_inode_mut(parent);
        let TempINode::Directory(parent_dir) = parent_inode else {
            panic!("Should never happen since we did this check above.");
        };
        parent_dir.entries.insert(name.into(), source.inode);
        Ok(())
    }
    fn symlink(&mut self, link: &Path, parent: FileHandle, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!(
                "tempfs: create symlink to {} in {}: {}",
                link, parent.inode, name
            );
        }
        // check for existence
        let parent_inode = self.get_inode(parent);
        let TempINode::Directory(parent_dir) = parent_inode else {
            panic!("Kernel should make sure parent is a directory before creating a link in it.");
        };
        if name.is_empty() || link.is_empty() {
            panic!("Empty path passed to symlink.");
        }
        if parent_dir.nlink == 0 {
            // this directory has been rmdir'd
            return Err(Error::NotDirectory);
        }
        if parent_dir.entries.contains_key(name) {
            return Err(Error::Exists);
        }
        let link_inode = TempINode::Link(TempLink {
            path: link.into(),
            nlink: 1,
        });
        let link_inode_num = self.add_inode(link_inode);
        // we can't just reuse parent_inode from above, since we accessed self in between.
        let parent_inode = self.get_inode_mut(parent);
        let TempINode::Directory(parent_dir) = parent_inode else {
            panic!("Should never happen since we did this check above.");
        };
        parent_dir.entries.insert(name.into(), link_inode_num);
        Ok(())
    }
    fn readlink(&self, link: FileHandle, buf: &mut Path) -> Result<Option<NonZeroUsize>> {
        if DEBUG_TEMPFS {
            println!("tempfs: readlink {} (buf len = {})", link.inode, buf.len());
        }
        let inode = self.get_inode(link);
        let TempINode::Link(link) = inode else {
            panic!(
                "Kernel should use stat to make sure this is a link before calling readlink on it."
            );
        };
        if buf.len() < link.path.len() {
            return Ok(None);
        }
        // unfortunately, unsafe code is currently the only way to write to a &mut str
        // SAFETY: we ensure that bytes is valid UTF-8 after this call,
        //         since link.path must be valid UTF-8.
        let bytes = unsafe { buf.as_bytes_mut() };
        bytes[..link.path.len()].copy_from_slice(link.path.as_bytes());
        for byte in &mut bytes[link.path.len()..] {
            if (*byte >> 6) == 0b10 {
                // replace continuation bytes following link.path with zeroes,
                // to ensure bytes remains valid UTF-8.
                *byte = 0;
            } else {
                break;
            }
        }
        Ok(Some(
            NonZeroUsize::new(link.path.len()).expect("symlink should have non-empty path"),
        ))
    }
    fn truncate(&mut self, file: FileHandle, size: u64) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: truncate {} to {} bytes", file.inode, size);
        }
        let inode = self.get_inode_mut(file);
        let TempINode::File(file) = inode else {
            panic!(
                "Kernel should use stat to make sure this is a file before calling truncate on it."
            );
        };
        if size <= file.data.len() as u64 {
            // shrink file
            file.data.truncate(size as usize);
        } else {
            // grow file
            let size: usize = size.try_into().map_err(|_| Error::NoSpace)?;
            let grow_by = size - file.data.len();
            file.data.try_reserve(grow_by).map_err(|_| Error::NoSpace)?;
            for _ in 0..grow_by {
                file.data.push(0);
            }
        }
        Ok(())
    }
    fn mkdir(&mut self, parent: FileHandle, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: mkdir in {}: {}", parent.inode, name);
        }
        if name.is_empty() {
            panic!("mkdir called with empty name");
        }
        let parent_inode = self.get_inode(parent);
        let TempINode::Directory(parent_dir) = parent_inode else {
            panic!(
                "Kernel should make sure parent is a directory before making a directory in it."
            );
        };
        if parent_dir.nlink == 0 {
            // this directory has been rmdir'd
            return Err(Error::NotDirectory);
        }
        if parent_dir.entries.contains_key(name) {
            return Err(Error::Exists);
        }
        let inode = TempINode::Directory(TempDirectory {
            entries: BTreeMap::new(),
            nlink: 1,
        });
        let inode_num = self.add_inode(inode);
        let parent_inode = self.get_inode_mut(parent);
        let TempINode::Directory(parent_dir) = parent_inode else {
            panic!("This should never happen due to the check above");
        };
        parent_dir.entries.insert(name.into(), inode_num);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // https://github.com/rust-lang/rust/pull/120234
    macro_rules! assert_matches {
        ($expression:expr, $pattern:pat) => {
            assert!(matches!($expression, $pattern))
        };
    }
    #[derive(Clone, Copy)]
    enum Action {
        Open,
        Create,
        Mkdir,
        Rmdir,
        Unlink,
        Link(FileHandle),
    }
    // open/create/mkdir/rmdir/unlink an absolute path
    fn do_path<F: FileSystem>(
        fs: &mut F,
        path: &Path,
        action: Action,
    ) -> Result<Option<FileHandle>> {
        if !path.starts_with("/") {
            panic!("not an absolute path");
        }
        let mut file = fs.open(fs.root())?;
        let component_count = path.split('/').count();
        for (i, item) in path.split('/').enumerate() {
            if item.is_empty() {
                continue;
            }
            if i == component_count - 1 {
                match action {
                    Action::Open => {}
                    Action::Create => {
                        return Ok(Some(fs.create(file, item)?));
                    }
                    Action::Mkdir => {
                        fs.mkdir(file, item)?;
                        return Ok(None);
                    }
                    Action::Rmdir => {
                        let inode = fs.lookup(file, item)?;
                        fs.rmdir(file, item)?;
                        fs.release(inode);
                        return Ok(None);
                    }
                    Action::Unlink => {
                        fs.unlink(file, item)?;
                        return Ok(None);
                    }
                    Action::Link(source) => {
                        fs.link(source, file, item)?;
                        return Ok(None);
                    }
                }
            }

            let next_file = fs.open(fs.lookup(file, item)?)?;
            file = next_file;
        }
        Ok(Some(file))
    }
    // mkdir an absolute path
    fn mkdir_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<()> {
        do_path(fs, path, Action::Mkdir)?;
        Ok(())
    }
    // create an absolute path
    fn create_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<FileHandle> {
        Ok(do_path(fs, path, Action::Create)?.unwrap())
    }
    // open an absolute path
    fn open_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<FileHandle> {
        Ok(do_path(fs, path, Action::Open)?.unwrap())
    }
    // rmdir an absolute path
    fn rmdir_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<()> {
        do_path(fs, path, Action::Rmdir)?;
        Ok(())
    }
    // unlink an absolute path
    fn unlink_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<()> {
        do_path(fs, path, Action::Unlink)?;
        Ok(())
    }
    // hard link an absolute path to an absolute path
    fn link_path<F: FileSystem>(fs: &mut F, source: &Path, dest: &Path) -> Result<()> {
        let source = open_path(fs, source)?;
        do_path(fs, dest, Action::Link(source))?;
        Ok(())
    }
    // read entire file contents
    fn read_file<F: FileSystem>(fs: &mut F, file: FileHandle) -> Result<Vec<u8>> {
        let mut buf = [0; 1024];
        let mut vec = Vec::new();
        loop {
            let n = fs.read(file, vec.len() as u64, &mut buf)?;
            if n == 0 {
                break;
            }
            vec.extend_from_slice(&buf[..n]);
        }
        return Ok(vec);
    }
    #[test]
    // one regular file in root
    fn simple_write_read() {
        let mut fs = TempFs::new();
        let test = create_path(&mut fs, "/test").unwrap();
        assert_eq!(fs.write(test, 0, b"hello").unwrap(), 5);
        fs.release(test.inode); // this should do nothing since there is still a link to /test
        let test = open_path(&mut fs, "/test").unwrap();
        assert_eq!(read_file(&mut fs, test).unwrap(), b"hello");
    }
    #[test]
    // test directories
    fn dirs() {
        let mut fs = TempFs::new();
        mkdir_path(&mut fs, "/dir1").unwrap();
        mkdir_path(&mut fs, "/dir2").unwrap();
        let foo = create_path(&mut fs, "/dir1/foo").unwrap();
        let bar = create_path(&mut fs, "/dir2/bar").unwrap();
        assert_eq!(fs.write(foo, 0, b"foo").unwrap(), 3);
        assert_eq!(fs.write(bar, 0, b"bar").unwrap(), 3);
        let foo = open_path(&mut fs, "/dir1/foo").unwrap();
        assert_eq!(read_file(&mut fs, foo).unwrap(), b"foo");
        let bar = open_path(&mut fs, "/dir2/bar").unwrap();
        assert_eq!(read_file(&mut fs, bar).unwrap(), b"bar");
        assert_matches!(
            open_path(&mut fs, "/dir1/bar").unwrap_err(),
            Error::NotFound
        );
        assert_matches!(
            open_path(&mut fs, "/dir2/foo").unwrap_err(),
            Error::NotFound
        );
        assert_matches!(open_path(&mut fs, "/dir3").unwrap_err(), Error::NotFound);
    }

    #[test]
    // test unlink
    fn unlink() {
        let mut fs = TempFs::new();
        mkdir_path(&mut fs, "/dir").unwrap();
        let file1 = create_path(&mut fs, "/dir/1").unwrap();
        assert_eq!(fs.write(file1, 0, b"test file").unwrap(), 9);
        create_path(&mut fs, "/2").unwrap();
        unlink_path(&mut fs, "/2").unwrap();
        assert_matches!(open_path(&mut fs, "/2").unwrap_err(), Error::NotFound);
        let file1 = open_path(&mut fs, "/dir/1").unwrap();
        unlink_path(&mut fs, "/dir/1").unwrap();
        assert_matches!(open_path(&mut fs, "/dir/1").unwrap_err(), Error::NotFound);
        // file data should still exist since there are open handles to it!
        assert_eq!(read_file(&mut fs, file1).unwrap(), b"test file");
        fs.release(file1.inode);
        assert_matches!(open_path(&mut fs, "/dir/1").unwrap_err(), Error::NotFound);
    }

    #[test]
    // test rmdir
    fn rmdir() {
        let mut fs = TempFs::new();
        mkdir_path(&mut fs, "/dir").unwrap();
        mkdir_path(&mut fs, "/dir/1").unwrap();
        mkdir_path(&mut fs, "/dir/1/2").unwrap();
        assert_matches!(rmdir_path(&mut fs, "/dir").unwrap_err(), Error::NotEmpty);
        rmdir_path(&mut fs, "/dir/1/2").unwrap();
        assert_matches!(open_path(&mut fs, "/dir/1/2").unwrap_err(), Error::NotFound);
        rmdir_path(&mut fs, "/dir/1").unwrap();
        assert_matches!(open_path(&mut fs, "/dir/1").unwrap_err(), Error::NotFound);
        rmdir_path(&mut fs, "/dir").unwrap();
        assert_matches!(open_path(&mut fs, "/dir").unwrap_err(), Error::NotFound);
        assert!(fs.inodes.len() == 1); // should only have root
    }

    #[test]
    // test link
    fn link() {
        let mut fs = TempFs::new();
        let one = create_path(&mut fs, "/1").unwrap();
        link_path(&mut fs, "/1", "/2").unwrap();
        link_path(&mut fs, "/2", "/3").unwrap();
        fs.write(one, 0, b"hello").unwrap();
        let two = open_path(&mut fs, "/2").unwrap();
        let three = open_path(&mut fs, "/3").unwrap();
        assert_eq!(read_file(&mut fs, two).unwrap(), b"hello");
        assert_eq!(read_file(&mut fs, three).unwrap(), b"hello");
        unlink_path(&mut fs, "/1").unwrap();
        fs.release(one.inode);
        assert_eq!(read_file(&mut fs, two).unwrap(), b"hello");
        unlink_path(&mut fs, "/2").unwrap();
        fs.release(two.inode);
        assert_eq!(read_file(&mut fs, three).unwrap(), b"hello");
        unlink_path(&mut fs, "/3").unwrap();
        fs.release(three.inode);
        assert!(fs.inodes.len() == 1); // should only have root
    }
}
