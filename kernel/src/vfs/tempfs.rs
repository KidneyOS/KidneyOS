#[cfg(not(test))]
use crate::println;
#[cfg(test)]
use std::println;

use crate::vfs::{
    DirEntry, DirectoryIterator, Error, FileHandle, FileInfo, FileSystem, INodeNum, INodeType,
    OwnedPath, Path, Result,
};
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::cmp::min;

#[derive(Default)]
struct TempFile {
    data: Vec<u8>,
}

#[derive(Default)]
struct TempDirectory {
    curr_id: u64,
    // unfortunately we need to keep track of a monotonically increasing ID,
    // to properly support the `offset` parameter to `FileSystem::readdir`.
    // (our readdir implementation returns the entries in the order they were
    //  added, with `TempDirectoryIterator::offset` returning the ID,
    //  so that we can meet all of the required guarantees for readdir).
    entry_by_id: BTreeMap<u64, (INodeNum, OwnedPath)>,
    id_by_name: BTreeMap<OwnedPath, u64>,
}

impl TempDirectory {
    fn inode_by_name(&self, name: &Path) -> Option<INodeNum> {
        let id = self.id_by_name.get(name)?;
        Some(
            self.entry_by_id
                .get(id)
                .expect("tempfs consistency error")
                .0,
        )
    }
    fn is_empty(&self) -> bool {
        self.entry_count() == 0
    }
    /// number of entries in directory
    fn entry_count(&self) -> usize {
        self.id_by_name.len()
    }
    fn contains(&self, path: &Path) -> bool {
        self.id_by_name.contains_key(path)
    }
    fn add_entry(&mut self, path: OwnedPath, inode: INodeNum) {
        self.curr_id += 1;
        let id = self.curr_id;
        self.entry_by_id.insert(id, (inode, path.clone()));
        self.id_by_name.insert(path, id);
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
    fn link_to(path: String) -> Self {
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
pub struct TempFs {
    inodes: BTreeMap<INodeNum, TempINode>,
    inode_counter: INodeNum,
}

const ROOT_INO: INodeNum = 1;

pub struct TempDirectoryIterator<'a> {
    fs: &'a TempFs,
    it: alloc::collections::btree_map::Range<'a, u64, (INodeNum, OwnedPath)>,
    offset: u64,
}

impl<'a> DirectoryIterator for TempDirectoryIterator<'a> {
    fn next(&mut self) -> Result<Option<DirEntry>> {
        let Some((id, (inode_num, name))) = self.it.next() else {
            return Ok(None);
        };
        let inode_num = *inode_num;
        let inode = self
            .fs
            .inodes
            .get(&inode_num)
            .expect("tempfs consistency error — reference to nonexistent inode");
        let r#type = inode.type_of();
        self.offset = *id;
        Ok(Some(DirEntry {
            inode: inode_num,
            r#type,
            name,
        }))
    }
    fn offset(&self) -> u64 {
        self.offset
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
        let root = TempINode::empty_directory();
        let mut inodes = BTreeMap::new();
        inodes.insert(ROOT_INO, root);
        TempFs {
            inodes,
            inode_counter: 1,
        }
    }
    fn get_inode(&self, handle: FileHandle) -> &TempINode {
        self.inodes.get(&handle.inode).expect(NO_INODE)
    }
    fn get_inode_mut(&mut self, handle: FileHandle) -> &mut TempINode {
        self.inodes.get_mut(&handle.inode).expect(NO_INODE)
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
    fn unlink_or_rmdir(&mut self, parent: FileHandle, name: &Path, is_rmdir: bool) -> Result<()> {
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
        let id = *parent_dir.id_by_name.get(name).ok_or(Error::NotFound)?;
        let inode_num = parent_dir
            .entry_by_id
            .get(&id)
            .expect("tempfs consistency error")
            .0;
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
        parent_dir.id_by_name.remove(name);
        parent_dir.entry_by_id.remove(&id);
        // Note that we don't actually remove the inode from self.inodes here;
        // we do that in `release`, so that existing file handles can still access
        // the file until then.
        Ok(())
    }
}

const DEBUG_TEMPFS: bool = cfg!(test);

impl FileSystem for TempFs {
    fn root(&self) -> INodeNum {
        ROOT_INO
    }
    fn lookup(&self, parent: FileHandle, name: &Path) -> Result<INodeNum> {
        if DEBUG_TEMPFS {
            println!("tempfs: lookup in {}: {}", parent.inode, name);
        }
        let parent_inode = self.get_inode(parent);
        let TempINodeData::Directory(dir) = &parent_inode.data else {
            return Err(Error::NotDirectory);
        };
        dir.inode_by_name(name).ok_or(Error::NotFound)
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
        Ok(FileHandle {
            inode: inode_num,
            fs_data: 0,
        })
    }
    fn unlink(&mut self, parent: FileHandle, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: unlink in {}: {}", parent.inode, name);
        }
        self.unlink_or_rmdir(parent, name, false)
    }
    fn rmdir(&mut self, parent: FileHandle, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: rmdir in {}: {}", parent.inode, name);
        }
        self.unlink_or_rmdir(parent, name, true)
    }
    fn readdir(&self, dir: FileHandle, offset: u64) -> impl '_ + DirectoryIterator {
        if DEBUG_TEMPFS {
            println!("tempfs: readdir {}", dir.inode);
        }
        let inode = self.get_inode(dir);
        let TempINodeData::Directory(dir) = &inode.data else {
            panic!("Kernel should call stat to make sure this is a directory before calling readdir on it.");
        };
        TempDirectoryIterator {
            offset: 0,
            fs: self,
            it: dir.entry_by_id.range(offset + 1..),
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
        if inode.nlink == 0 {
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
    fn stat(&self, file: FileHandle) -> Result<FileInfo> {
        if DEBUG_TEMPFS {
            println!("tempfs: stat {}", file.inode);
        }
        let inode = self.get_inode(file);
        match &inode.data {
            TempINodeData::Directory(d) => Ok(FileInfo {
                r#type: INodeType::Directory,
                inode: file.inode,
                nlink: inode.nlink,
                // pretend that each entry takes up 16 bytes (chosen arbitrarily)
                size: d.entry_count() as u64 * 16,
            }),
            TempINodeData::File(f) => Ok(FileInfo {
                r#type: INodeType::File,
                inode: file.inode,
                nlink: inode.nlink,
                size: f.data.len() as u64,
            }),
            TempINodeData::Link(l) => Ok(FileInfo {
                r#type: INodeType::Link,
                inode: file.inode,
                nlink: inode.nlink,
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
        parent_dir.add_entry(name.into(), source.inode);
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
    fn readlink<'a>(&self, link: FileHandle, buf: &'a mut str) -> Result<Option<&'a str>> {
        if DEBUG_TEMPFS {
            println!("tempfs: readlink {} (buf len = {})", link.inode, buf.len());
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
        // unfortunately, unsafe code is currently the only way to write to a &mut str
        // SAFETY: we ensure that bytes is valid UTF-8 after readlink returns,
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
        Ok(Some(&buf[..link.path.len()]))
    }
    fn truncate(&mut self, file: FileHandle, size: u64) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: truncate {} to {} bytes", file.inode, size);
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
    fn mkdir(&mut self, parent: FileHandle, name: &Path) -> Result<()> {
        if DEBUG_TEMPFS {
            println!("tempfs: mkdir in {}: {}", parent.inode, name);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs;

    // https://github.com/rust-lang/rust/pull/120234
    macro_rules! assert_matches {
        ($expression:expr, $pattern:pat) => {
            assert!(matches!($expression, $pattern))
        };
    }
    #[derive(Clone, Copy)]
    enum Action<'a> {
        Open,
        Create,
        Mkdir,
        Rmdir,
        Unlink,
        Link(FileHandle),
        SymLink(&'a Path),
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
                    Action::SymLink(source) => {
                        fs.symlink(source, file, item)?;
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
    // sym link to an absolute path
    fn symlink_path<F: FileSystem>(fs: &mut F, source: &Path, dest: &Path) -> Result<()> {
        do_path(fs, dest, Action::SymLink(source))?;
        Ok(())
    }
    // read link from an absolute path
    fn readlink_path<F: FileSystem>(fs: &mut F, source: &Path) -> Result<OwnedPath> {
        let file = do_path(fs, source, Action::Open)?.unwrap();
        let mut buf = OwnedPath::new();
        loop {
            if let Some(s) = fs.readlink(file, &mut buf)? {
                return Ok(s.into());
            }
            buf.push('\0');
        }
    }
    // get inode of absolute path
    fn inode_of_path<F: FileSystem>(fs: &mut F, path: &Path) -> Result<INodeNum> {
        Ok(open_path(fs, path)?.inode)
    }
    // get directory entries sorted by name
    fn readdir_path<'a, F: FileSystem>(
        fs: &'a mut F,
        path: &Path,
    ) -> Result<Vec<vfs::OwnedDirEntry>> {
        let handle = open_path(fs, path)?;
        let mut it = fs.readdir(handle, 0);
        let mut v = vec![];
        let mut i = 0;
        while let Some(entry) = it.next()? {
            v.push(entry.to_owned());
            if i % 2 == 0 {
                // test readdir with offset
                it = fs.readdir(handle, it.offset());
            }
            i += 1;
        }
        v.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(v)
    }
    // read entire file contents
    fn read_file<F: FileSystem>(fs: &F, file: FileHandle) -> Result<Vec<u8>> {
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
        assert_eq!(fs.inodes.len(), 1); // should only have root
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
        assert_eq!(fs.inodes.len(), 1); // should only have root
    }

    #[test]
    // test symlink, readlink
    fn symlink() {
        let mut fs = TempFs::new();
        symlink_path(&mut fs, "/file", "/1").unwrap();
        symlink_path(&mut fs, "./file", "/2").unwrap();
        symlink_path(&mut fs, "foo", "/3").unwrap();
        assert_eq!(readlink_path(&mut fs, "/1").unwrap(), "/file");
        assert_eq!(readlink_path(&mut fs, "/2").unwrap(), "./file");
        assert_eq!(readlink_path(&mut fs, "/3").unwrap(), "foo");
    }

    #[test]
    #[ignore = "slow"]
    /// test TooManyLinks error
    fn too_many_links() {
        let mut fs = TempFs::new();
        let source = create_path(&mut fs, "/file").unwrap();
        let root = open_path(&mut fs, "/").unwrap();
        for i in 0..65534 {
            fs.link(source, root, &format!("/{i}")).unwrap();
        }
        assert_matches!(
            fs.link(source, root, &format!("/65535")).unwrap_err(),
            Error::TooManyLinks
        );
    }

    #[test]
    fn stat() {
        let mut fs = TempFs::new();
        mkdir_path(&mut fs, "/dir").unwrap();
        symlink_path(&mut fs, "/dir", "/symlink").unwrap();
        let file = create_path(&mut fs, "/file").unwrap();
        link_path(&mut fs, "/file", "/hardlink").unwrap();
        let file2 = open_path(&mut fs, "/hardlink").unwrap();
        let symlink = open_path(&mut fs, "/symlink").unwrap();
        let dir = open_path(&mut fs, "/dir").unwrap();
        fs.write(file, 0, b"testing").unwrap();
        let file_stat = fs.stat(file).unwrap();
        let file2_stat = fs.stat(file2).unwrap();
        let dir_stat = fs.stat(dir).unwrap();
        let symlink_stat = fs.stat(symlink).unwrap();
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
        let mut fs = TempFs::new();
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
        let mut expect_entry = |entry: &vfs::OwnedDirEntry, r#type: INodeType, path: &Path| {
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
        let mut fs = TempFs::new();
        let test_file = create_path(&mut fs, "/test").unwrap();
        assert_eq!(
            fs.write(test_file, 0, b"hello world").unwrap(),
            b"hello world".len()
        );
        assert_eq!(read_file(&fs, test_file).unwrap(), b"hello world");
        fs.truncate(test_file, 5).unwrap();
        assert_eq!(read_file(&fs, test_file).unwrap(), b"hello");
        fs.truncate(test_file, 10).unwrap();
        assert_eq!(read_file(&fs, test_file).unwrap(), b"hello\0\0\0\0\0");
    }
}
