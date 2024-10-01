#[cfg(not(test))]
use crate::println;
#[cfg(test)]
use std::println;

use crate::vfs::{
    DirEntries, Error, FileInfo, INodeNum, INodeType, OwnedPath, Path, Result, SimpleFileSystem,
};
use alloc::{collections::BTreeMap, vec::Vec};
use core::cmp::min;

#[derive(Default)]
struct TempFile {
    data: Vec<u8>,
}

#[derive(Default)]
struct TempDirectory {
    entries: BTreeMap<OwnedPath, INodeNum>,
}

impl TempDirectory {
    fn is_empty(&self) -> bool {
        self.entry_count() == 0
    }
    /// number of entries in directory
    fn entry_count(&self) -> usize {
        self.entries.len()
    }
    fn contains(&self, path: &Path) -> bool {
        self.entries.contains_key(path)
    }
    fn add_entry(&mut self, path: OwnedPath, inode: INodeNum) {
        self.entries.insert(path, inode);
    }
    fn inode_by_name(&self, name: &Path) -> Option<INodeNum> {
        self.entries.get(name).copied()
    }
    fn remove(&mut self, name: &Path) {
        self.entries.remove(name);
    }
}

struct TempLink {
    path: OwnedPath,
}

enum TempINodeData {
    File(TempFile),
    Directory(TempDirectory),
    Link(TempLink),
}

struct TempINode {
    nlink: u16,
    data: TempINodeData,
    // could add mode, owner, etc. here
}

impl TempINode {
    fn new(data: TempINodeData) -> Self {
        Self { nlink: 1, data }
    }
    fn empty_directory() -> Self {
        Self::new(TempINodeData::Directory(TempDirectory::default()))
    }
    fn empty_file() -> Self {
        Self::new(TempINodeData::File(TempFile { data: Vec::new() }))
    }
    fn link_to(path: OwnedPath) -> Self {
        Self::new(TempINodeData::Link(TempLink { path }))
    }
    fn type_of(&self) -> INodeType {
        match &self.data {
            TempINodeData::File(_) => INodeType::File,
            TempINodeData::Directory(_) => INodeType::Directory,
            TempINodeData::Link(_) => INodeType::Link,
        }
    }
}

/// in-memory filesystem
pub struct TempFS {
    inodes: BTreeMap<INodeNum, TempINode>,
    inode_counter: INodeNum,
}

const ROOT_INO: INodeNum = 1;

impl Default for TempFS {
    fn default() -> Self {
        Self::new()
    }
}

const NO_INODE: &str = "Couldn't find inode — either kernel is using filesystem incorrectly or we freed an inode when we shouldn't have.";
impl TempFS {
    pub fn new() -> TempFS {
        let root = TempINode::empty_directory();
        let mut inodes = BTreeMap::new();
        inodes.insert(ROOT_INO, root);
        TempFS {
            inodes,
            inode_counter: 1,
        }
    }
    fn get_inode(&self, inode: INodeNum) -> &TempINode {
        self.inodes.get(&inode).expect(NO_INODE)
    }
    fn get_inode_mut(&mut self, inode: INodeNum) -> &mut TempINode {
        self.inodes.get_mut(&inode).expect(NO_INODE)
    }
    fn add_inode(&mut self, inode: TempINode) -> INodeNum {
        loop {
            self.inode_counter = self.inode_counter.wrapping_add(1);
            if !self.inodes.contains_key(&self.inode_counter) {
                break;
            }
        }
        self.inodes.insert(self.inode_counter, inode);
        self.inode_counter
    }
    // performs either unlink or rmdir.
    fn unlink_or_rmdir(&mut self, parent: INodeNum, name: &Path, is_rmdir: bool) -> Result<()> {
        if name.is_empty() {
            panic!(
                "Empty name passed to {}",
                if is_rmdir { "rmdir" } else { "unlink" }
            );
        }
        if name.contains('/') {
            panic!("File name contains /");
        }
        let parent_inode = self.get_inode(parent);
        let TempINodeData::Directory(parent_dir) = &parent_inode.data else {
            panic!("Kernel should call stat to make sure this is a directory before removing something from it.");
        };
        let inode_num = parent_dir
            .inode_by_name(name)
            .expect("tempfs consistency error");
        let inode = self
            .inodes
            .get_mut(&inode_num)
            .expect("inconsistent filesystem state — referenced inode doesn't exist");
        match &inode.data {
            TempINodeData::Directory(d) => {
                if !is_rmdir {
                    return Err(Error::NotDirectory);
                }
                if !d.is_empty() {
                    return Err(Error::NotEmpty);
                }
            }
            TempINodeData::File(_) => {
                if is_rmdir {
                    return Err(Error::NotDirectory);
                }
            }
            TempINodeData::Link(_) => {
                if is_rmdir {
                    return Err(Error::NotDirectory);
                }
            }
        }
        assert!(inode.nlink > 0, "removing a file with 0 links");
        inode.nlink -= 1;
        let parent_inode = self.get_inode_mut(parent);
        let TempINodeData::Directory(parent_dir) = &mut parent_inode.data else {
            panic!("This should never happen due to check above.");
        };
        // remove directory entry
        parent_dir.remove(name);
        // Note that we don't actually remove the inode from self.inodes here;
        // we do that in `release`, so that existing file handles can still access
        // the file until then.
        Ok(())
    }
}

const DEBUG_TEMPFS: bool = cfg!(test);

impl SimpleFileSystem for TempFS {
    fn root(&self) -> INodeNum {
        ROOT_INO
    }
    fn open(&mut self, inode: INodeNum) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: open {inode}");
        }
        if self.inodes.get(&inode).is_none() {
            return Err(Error::NotFound);
        }
        Ok(())
    }
    fn create(&mut self, parent: INodeNum, name: &Path) -> Result<INodeNum> {
        if DEBUG_TEMPFS {
            println!("tempfs: create in {parent:?}: {name}");
        }
        if name.is_empty() {
            panic!("Empty name passed to create");
        }
        if name.contains('/') {
            panic!("File name contains /");
        }
        let parent_inode = self.get_inode_mut(parent);
        if parent_inode.nlink == 0 {
            // this directory has been rmdir'd
            return Err(Error::NotDirectory);
        }
        let TempINodeData::Directory(parent_dir) = &mut parent_inode.data else {
            panic!("Kernel should call stat to make sure this is a directory before creating a file in it.");
        };
        let inode_num = parent_dir.inode_by_name(name).unwrap_or_else(|| {
            // create new file
            let inode_num = self.add_inode(TempINode::empty_file());
            let parent_inode = self.get_inode_mut(parent);
            let TempINodeData::Directory(parent_dir) = &mut parent_inode.data else {
                panic!("should never happen due to check above");
            };
            parent_dir.add_entry(name.into(), inode_num);
            inode_num
        });
        Ok(inode_num)
    }
    fn unlink(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: unlink in {parent:?}: {name}");
        }
        self.unlink_or_rmdir(parent, name, false)
    }
    fn rmdir(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: rmdir in {parent:?}: {name}");
        }
        self.unlink_or_rmdir(parent, name, true)
    }
    fn readdir(&mut self, dir: INodeNum) -> Result<DirEntries> {
        if DEBUG_TEMPFS {
            println!("tempfs: readdir {dir:?}");
        }
        let inode = self.get_inode(dir);
        let TempINodeData::Directory(dir) = &inode.data else {
            panic!("Kernel should call stat to make sure this is a directory before calling readdir on it.");
        };
        let mut entries = DirEntries::new();
        for (name, inode_num) in dir.entries.iter() {
            let inode = self
                .inodes
                .get(inode_num)
                .expect("tempfs consistency error — referencing free inode");
            entries.add(*inode_num, inode.type_of(), name);
        }
        Ok(entries)
    }
    fn release(&mut self, inode_num: INodeNum) {
        if DEBUG_TEMPFS {
            println!("tempfs: release {inode_num}");
        }
        let inode = self
            .inodes
            .get(&inode_num)
            .expect("kernel should only call release on inodes it knows to exist");
        if inode.nlink == 0 {
            // we can safely remove the inode.
            self.inodes.remove(&inode_num);
        }
    }
    fn read(&mut self, file: INodeNum, offset: u64, buf: &mut [u8]) -> Result<usize> {
        if DEBUG_TEMPFS {
            println!(
                "tempfs: read from {file:?} @ offset {offset} length {}",
                buf.len()
            );
        }
        let inode = self.get_inode(file);
        let TempINodeData::File(f) = &inode.data else {
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
    fn write(&mut self, file: INodeNum, offset: u64, buf: &[u8]) -> Result<usize> {
        if DEBUG_TEMPFS {
            println!(
                "tempfs: write to {file:?} @ offset {offset} length {}",
                buf.len()
            );
        }
        let inode = self.get_inode_mut(file);
        let TempINodeData::File(f) = &mut inode.data else {
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
    fn stat(&mut self, file: INodeNum) -> Result<FileInfo> {
        if DEBUG_TEMPFS {
            println!("tempfs: stat {file:?}");
        }
        let inode = self.get_inode(file);
        match &inode.data {
            TempINodeData::Directory(d) => Ok(FileInfo {
                r#type: INodeType::Directory,
                inode: file,
                nlink: inode.nlink.into(),
                // pretend that each entry takes up 16 bytes (chosen arbitrarily)
                size: d.entry_count() as u64 * 16,
            }),
            TempINodeData::File(f) => Ok(FileInfo {
                r#type: INodeType::File,
                inode: file,
                nlink: inode.nlink.into(),
                size: f.data.len() as u64,
            }),
            TempINodeData::Link(l) => Ok(FileInfo {
                r#type: INodeType::Link,
                inode: file,
                nlink: inode.nlink.into(),
                size: l.path.len() as u64,
            }),
        }
    }
    fn link(&mut self, source: INodeNum, parent: INodeNum, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: create link to {source:?} in {parent:?}: {name}",);
        }
        // check for existence
        let parent_inode = self.get_inode(parent);
        let TempINodeData::Directory(parent_dir) = &parent_inode.data else {
            panic!("Kernel should make sure parent is a directory via stat before creating a link in it.");
        };
        if parent_inode.nlink == 0 {
            // this directory has been rmdir'd
            return Err(Error::NotFound);
        }
        if parent_dir.contains(name) {
            return Err(Error::Exists);
        }
        // increment link count
        let source_inode = self.get_inode_mut(source);
        source_inode.nlink = source_inode
            .nlink
            .checked_add(1)
            .ok_or(Error::TooManyLinks)?;
        // insert directory entry
        let parent_inode = self.get_inode_mut(parent);
        let TempINodeData::Directory(parent_dir) = &mut parent_inode.data else {
            panic!("Should never happen since we did this check above.");
        };
        parent_dir.add_entry(name.into(), source);
        Ok(())
    }
    fn symlink(&mut self, link: &Path, parent: INodeNum, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: create symlink to {link} in {parent:?}: {name}",);
        }
        // check for existence
        let parent_inode = self.get_inode(parent);
        let TempINodeData::Directory(parent_dir) = &parent_inode.data else {
            panic!("Kernel should make sure parent is a directory via stat before creating a symlink in it.");
        };
        if name.is_empty() || link.is_empty() {
            panic!("Empty path passed to symlink.");
        }
        if name.contains('/') {
            panic!("File name contains /");
        }
        if parent_inode.nlink == 0 {
            // this directory has been rmdir'd
            return Err(Error::NotFound);
        }
        if parent_dir.contains(name) {
            return Err(Error::Exists);
        }
        let link_inode = TempINode::link_to(link.into());
        let link_inode_num = self.add_inode(link_inode);
        let parent_inode = self.get_inode_mut(parent);
        let TempINodeData::Directory(parent_dir) = &mut parent_inode.data else {
            panic!("Should never happen since we did this check above.");
        };
        parent_dir.add_entry(name.into(), link_inode_num);
        Ok(())
    }
    fn readlink_no_alloc<'a>(
        &mut self,
        link: INodeNum,
        buf: &'a mut [u8],
    ) -> Result<Option<&'a str>> {
        if DEBUG_TEMPFS {
            println!("tempfs: readlink {link:?} (buf len = {})", buf.len());
        }
        let inode = self.get_inode(link);
        let TempINodeData::Link(link) = &inode.data else {
            panic!(
                "Kernel should use stat to make sure this is a link before calling readlink on it."
            );
        };
        if buf.len() < link.path.len() {
            return Ok(None);
        }
        buf[..link.path.len()].copy_from_slice(link.path.as_bytes());
        Ok(Some(core::str::from_utf8(&buf[..link.path.len()]).expect(
            "should be valid UTF-8 since it was copied from a str",
        )))
    }
    fn truncate(&mut self, file: INodeNum, size: u64) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: truncate {file:?} to {size} bytes");
        }
        let inode = self.get_inode_mut(file);
        let TempINodeData::File(file) = &mut inode.data else {
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
    fn mkdir(&mut self, parent: INodeNum, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: mkdir in {parent:?}: {name}");
        }
        if name.is_empty() {
            panic!("mkdir called with empty name");
        }
        if name.contains('/') {
            panic!("File name contains /");
        }
        let parent_inode = self.get_inode(parent);
        let TempINodeData::Directory(parent_dir) = &parent_inode.data else {
            panic!(
                "Kernel should make sure parent is a directory before making a directory in it."
            );
        };
        if parent_inode.nlink == 0 {
            // this directory has been rmdir'd
            return Err(Error::NotDirectory);
        }
        if parent_dir.contains(name) {
            return Err(Error::Exists);
        }
        let inode = TempINode::empty_directory();
        let inode_num = self.add_inode(inode);
        let parent_inode = self.get_inode_mut(parent);
        let TempINodeData::Directory(parent_dir) = &mut parent_inode.data else {
            panic!("This should never happen due to the check above");
        };
        parent_dir.add_entry(name.into(), inode_num);
        Ok(())
    }
    fn sync(&mut self) -> Result<()> {
        // not applicable to in-memory filesystem
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::TempFS;
    use crate::vfs::{
        Error, FileHandle, FileSystem, INodeNum, INodeType, OwnedDirEntry, OwnedPath, Path, Result,
    };

    // https://github.com/rust-lang/rust/pull/120234
    macro_rules! assert_matches {
        ($expression:expr, $pattern:pat) => {
            assert!(matches!($expression, $pattern))
        };
    }
    // NOTE: this is quite inefficient and should only be used for testing!
    fn lookup<F: FileSystem>(
        fs: &mut F,
        parent: &mut F::FileHandle,
        name: &str,
    ) -> Result<INodeNum> {
        let entries = fs.readdir(parent)?;
        for entry in entries.into_iter() {
            if entry.name == name {
                return Ok(entry.inode);
            }
        }
        Err(Error::NotFound)
    }
    #[derive(Clone, Copy)]
    enum Action<'a, F: FileSystem> {
        Open,
        Create,
        Mkdir,
        Rmdir,
        Unlink,
        Link(F::FileHandle),
        SymLink(&'a Path),
    }
    // open/create/mkdir/rmdir/unlink an absolute path
    fn do_path<F: FileSystem>(
        fs: &mut F,
        path: &Path,
        action: Action<F>,
    ) -> Result<Option<F::FileHandle>> {
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
                        return Ok(Some(fs.create(&mut file, item)?));
                    }
                    Action::Mkdir => {
                        fs.mkdir(&mut file, item)?;
                        return Ok(None);
                    }
                    Action::Rmdir => {
                        let inode = lookup(fs, &mut file, item)?;
                        fs.rmdir(&mut file, item)?;
                        fs.release(inode);
                        return Ok(None);
                    }
                    Action::Unlink => {
                        fs.unlink(&mut file, item)?;
                        return Ok(None);
                    }
                    Action::Link(mut source) => {
                        fs.link(&mut source, &mut file, item)?;
                        return Ok(None);
                    }
                    Action::SymLink(source) => {
                        fs.symlink(source, &mut file, item)?;
                        return Ok(None);
                    }
                }
            }
            let inode = lookup(fs, &mut file, item)?;
            file = fs.open(inode)?;
        }
        Ok(Some(file))
    }
    // mkdir an absolute path
    fn mkdir_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<()> {
        do_path(fs, path, Action::Mkdir)?;
        Ok(())
    }
    // create an absolute path
    fn create_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<F::FileHandle> {
        Ok(do_path(fs, path, Action::Create)?.unwrap())
    }
    // open an absolute path
    fn open_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<F::FileHandle> {
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
    // sym link to an absolute path
    fn symlink_path<F: FileSystem>(fs: &mut F, source: &Path, dest: &Path) -> Result<()> {
        do_path(fs, dest, Action::SymLink(source))?;
        Ok(())
    }
    // read link from an absolute path
    fn readlink_path<F: FileSystem>(fs: &mut F, source: &Path) -> Result<OwnedPath> {
        let mut file = do_path(fs, source, Action::Open)?.unwrap();
        let mut buf: Vec<u8> = Vec::new();
        loop {
            if let Some(s) = fs.readlink(&mut file, &mut buf)? {
                return Ok(s.into());
            }
            buf.push(0);
        }
    }
    // get inode of absolute path
    fn inode_of_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<INodeNum> {
        Ok(open_path(fs, path)?.inode())
    }
    // get directory entries sorted by name
    fn readdir_path<'a, F: FileSystem>(fs: &'a mut F, path: &Path) -> Result<Vec<OwnedDirEntry>> {
        let mut handle = open_path(fs, path)?;
        Ok(fs.readdir(&mut handle)?.to_sorted_vec())
    }
    // read entire file contents
    fn read_file<F: FileSystem>(fs: &mut F, file: &mut F::FileHandle) -> Result<Vec<u8>> {
        let mut buf = [0; 2]; // use just 2 bytes for buffer for more thorough testing
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
        let mut fs = TempFS::new();
        let mut test = create_path(&mut fs, "/test").unwrap();
        assert_eq!(fs.write(&mut test, 0, b"hello").unwrap(), 5);
        fs.release(test.inode()); // this should do nothing since there is still a link to /test
        let mut test = open_path(&mut fs, "/test").unwrap();
        assert_eq!(read_file(&mut fs, &mut test).unwrap(), b"hello");
    }
    #[test]
    // test directories
    fn dirs() {
        let mut fs = TempFS::new();
        mkdir_path(&mut fs, "/dir1").unwrap();
        mkdir_path(&mut fs, "/dir2").unwrap();
        let mut foo = create_path(&mut fs, "/dir1/foo").unwrap();
        let mut bar = create_path(&mut fs, "/dir2/bar").unwrap();
        assert_eq!(fs.write(&mut foo, 0, b"foo").unwrap(), 3);
        assert_eq!(fs.write(&mut bar, 0, b"bar").unwrap(), 3);
        let mut foo = open_path(&mut fs, "/dir1/foo").unwrap();
        assert_eq!(read_file(&mut fs, &mut foo).unwrap(), b"foo");
        let mut bar = open_path(&mut fs, "/dir2/bar").unwrap();
        assert_eq!(read_file(&mut fs, &mut bar).unwrap(), b"bar");
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
        let mut fs = TempFS::new();
        mkdir_path(&mut fs, "/dir").unwrap();
        let mut file1 = create_path(&mut fs, "/dir/1").unwrap();
        assert_eq!(fs.write(&mut file1, 0, b"test file").unwrap(), 9);
        create_path(&mut fs, "/2").unwrap();
        unlink_path(&mut fs, "/2").unwrap();
        assert_matches!(open_path(&mut fs, "/2").unwrap_err(), Error::NotFound);
        let mut file1 = open_path(&mut fs, "/dir/1").unwrap();
        unlink_path(&mut fs, "/dir/1").unwrap();
        assert_matches!(open_path(&mut fs, "/dir/1").unwrap_err(), Error::NotFound);
        // file data should still exist since there are open handles to it!
        assert_eq!(read_file(&mut fs, &mut file1).unwrap(), b"test file");
        fs.release(file1.inode());
        assert_matches!(open_path(&mut fs, "/dir/1").unwrap_err(), Error::NotFound);
    }

    #[test]
    // test rmdir
    fn rmdir() {
        let mut fs = TempFS::new();
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
        assert_eq!(fs.inodes.len(), 1); // should only have root
    }

    #[test]
    // test link
    fn link() {
        let mut fs = TempFS::new();
        let mut one = create_path(&mut fs, "/1").unwrap();
        link_path(&mut fs, "/1", "/2").unwrap();
        link_path(&mut fs, "/2", "/3").unwrap();
        fs.write(&mut one, 0, b"hello").unwrap();
        let mut two = open_path(&mut fs, "/2").unwrap();
        let mut three = open_path(&mut fs, "/3").unwrap();
        assert_eq!(read_file(&mut fs, &mut two).unwrap(), b"hello");
        assert_eq!(read_file(&mut fs, &mut three).unwrap(), b"hello");
        unlink_path(&mut fs, "/1").unwrap();
        fs.release(one.inode());
        assert_eq!(read_file(&mut fs, &mut two).unwrap(), b"hello");
        unlink_path(&mut fs, "/2").unwrap();
        fs.release(two.inode());
        assert_eq!(read_file(&mut fs, &mut three).unwrap(), b"hello");
        unlink_path(&mut fs, "/3").unwrap();
        fs.release(three.inode());
        assert_eq!(fs.inodes.len(), 1); // should only have root
    }

    #[test]
    // test symlink, readlink
    fn symlink() {
        let mut fs = TempFS::new();
        symlink_path(&mut fs, "/file", "/1").unwrap();
        symlink_path(&mut fs, "./file", "/2").unwrap();
        symlink_path(&mut fs, "foo", "/3").unwrap();
        assert_eq!(readlink_path(&mut fs, "/1").unwrap(), "/file");
        assert_eq!(readlink_path(&mut fs, "/2").unwrap(), "./file");
        assert_eq!(readlink_path(&mut fs, "/3").unwrap(), "foo");
    }

    #[test]
    fn stat() {
        let mut fs = TempFS::new();
        mkdir_path(&mut fs, "/dir").unwrap();
        symlink_path(&mut fs, "/dir", "/symlink").unwrap();
        let mut file = create_path(&mut fs, "/file").unwrap();
        link_path(&mut fs, "/file", "/hardlink").unwrap();
        let file2 = open_path(&mut fs, "/hardlink").unwrap();
        let symlink = open_path(&mut fs, "/symlink").unwrap();
        let dir = open_path(&mut fs, "/dir").unwrap();
        fs.write(&mut file, 0, b"testing").unwrap();
        let file_stat = fs.stat(&file).unwrap();
        let file2_stat = fs.stat(&file2).unwrap();
        let dir_stat = fs.stat(&dir).unwrap();
        let symlink_stat = fs.stat(&symlink).unwrap();
        assert_eq!(file_stat.r#type, INodeType::File);
        assert_eq!(file2_stat.r#type, INodeType::File);
        assert_eq!(dir_stat.r#type, INodeType::Directory);
        assert_eq!(symlink_stat.r#type, INodeType::Link);
        assert_eq!(file_stat.size, 7);
        assert_eq!(file2_stat.size, 7);
        assert_eq!(symlink_stat.size, 4);
        assert_ne!(file_stat.inode, dir_stat.inode);
        assert_ne!(file_stat.inode, symlink_stat.inode);
        assert_ne!(dir_stat.inode, symlink_stat.inode);
        assert_eq!(file_stat.inode, file2_stat.inode);
        assert_eq!(dir_stat.nlink, 1);
        assert_eq!(symlink_stat.nlink, 1);
        assert_eq!(file_stat.nlink, 2);
        assert_eq!(file2_stat.nlink, 2);
    }

    #[test]
    fn readdir() {
        let mut fs = TempFS::new();
        mkdir_path(&mut fs, "/dir").unwrap();
        create_path(&mut fs, "/dir/a").unwrap();
        create_path(&mut fs, "/dir/b").unwrap();
        create_path(&mut fs, "/dir/c").unwrap();
        create_path(&mut fs, "/dir/d").unwrap();
        create_path(&mut fs, "/dir/e").unwrap();
        symlink_path(&mut fs, "foo", "/dir/s").unwrap();
        create_path(&mut fs, "/f").unwrap();
        let root_entries = readdir_path(&mut fs, "/").unwrap();
        let dir_entries = readdir_path(&mut fs, "/dir").unwrap();
        let mut expect_entry = |entry: &OwnedDirEntry, r#type: INodeType, path: &Path| {
            assert_eq!(entry.r#type, r#type);
            assert_eq!(entry.name, path.rsplit_once('/').unwrap().1);
            assert_eq!(entry.inode, inode_of_path(&mut fs, path).unwrap());
        };
        assert_eq!(root_entries.len(), 2);
        expect_entry(&root_entries[0], INodeType::Directory, "/dir");
        expect_entry(&root_entries[1], INodeType::File, "/f");
        assert_eq!(dir_entries.len(), 6);
        expect_entry(&dir_entries[0], INodeType::File, "/dir/a");
        expect_entry(&dir_entries[1], INodeType::File, "/dir/b");
        expect_entry(&dir_entries[2], INodeType::File, "/dir/c");
        expect_entry(&dir_entries[3], INodeType::File, "/dir/d");
        expect_entry(&dir_entries[4], INodeType::File, "/dir/e");
        expect_entry(&dir_entries[5], INodeType::Link, "/dir/s");
    }

    #[test]
    fn truncate() {
        let mut fs = TempFS::new();
        let mut test_file = create_path(&mut fs, "/test").unwrap();
        assert_eq!(
            fs.write(&mut test_file, 0, b"hello world").unwrap(),
            b"hello world".len()
        );
        assert_eq!(read_file(&mut fs, &mut test_file).unwrap(), b"hello world");
        fs.truncate(&mut test_file, 5).unwrap();
        assert_eq!(read_file(&mut fs, &mut test_file).unwrap(), b"hello");
        fs.truncate(&mut test_file, 10).unwrap();
        assert_eq!(
            read_file(&mut fs, &mut test_file).unwrap(),
            b"hello\0\0\0\0\0"
        );
    }
}
