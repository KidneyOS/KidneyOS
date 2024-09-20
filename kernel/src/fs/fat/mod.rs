use crate::block::block_core::Block;
use crate::vfs::{DirectoryIterator, FileHandle, FileInfo, FileSystem, INodeNum, Path, Result};

pub struct FatFS {
    // underlying block device
    block: Block,
}

#[derive(Clone, Copy)]
pub struct FatFileHandle {
    inode: INodeNum,
}

impl FileHandle for FatFileHandle {
    fn inode(self) -> INodeNum {
        self.inode
    }
}

pub struct FatDirectoryIterator<'a> {
    _foo: core::marker::PhantomData<&'a u8>,
}

impl DirectoryIterator for FatDirectoryIterator<'_> {
    fn next(&mut self) -> Result<Option<crate::vfs::DirEntry<'_>>> {
        todo!()
    }
    fn offset(&self) -> u64 {
        todo!()
    }
}

impl FatFS {
    /// Create new FAT filesystem from block device
    pub fn new(block: Block) -> Self {
        Self { block }
    }
}

impl FileSystem for FatFS {
    type FileHandle = FatFileHandle;
    type DirectoryIterator<'a> = FatDirectoryIterator<'a>;
    fn root(&self) -> INodeNum {
        let _ = self.block;
        todo!()
    }
    fn lookup(&self, _parent: Self::FileHandle, _name: &Path) -> Result<INodeNum> {
        todo!()
    }
    fn open(&mut self, _inode: INodeNum) -> Result<Self::FileHandle> {
        todo!()
    }
    fn create(&mut self, _parent: Self::FileHandle, _name: &Path) -> Result<Self::FileHandle> {
        todo!()
    }
    fn mkdir(&mut self, _parent: Self::FileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn unlink(&mut self, _parent: Self::FileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn rmdir(&mut self, _parent: Self::FileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn readdir(&self, _dir: Self::FileHandle, _offset: u64) -> Self::DirectoryIterator<'_> {
        todo!()
    }
    fn release(&mut self, _inode: INodeNum) {
        todo!()
    }
    fn read(&self, _file: Self::FileHandle, _offset: u64, _buf: &mut [u8]) -> Result<usize> {
        todo!()
    }
    fn write(&mut self, _file: Self::FileHandle, _offset: u64, _buf: &[u8]) -> Result<usize> {
        todo!()
    }
    fn stat(&self, _file: Self::FileHandle) -> Result<FileInfo> {
        todo!()
    }
    fn link(
        &mut self,
        _source: Self::FileHandle,
        _parent: Self::FileHandle,
        _name: &Path,
    ) -> Result<()> {
        todo!()
    }
    fn symlink(&mut self, _link: &Path, _parent: Self::FileHandle, _name: &Path) -> Result<()> {
        todo!()
    }
    fn readlink<'a>(&self, _link: Self::FileHandle, _buf: &'a mut Path) -> Result<Option<&'a str>> {
        todo!()
    }
    fn truncate(&mut self, _file: Self::FileHandle, _size: u64) -> Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::block::block_core::test::block_from_file;
    use std::fs::File;
    use std::io::{prelude::*, Cursor};
    /// Open a gzip-compressed raw disk image containing a FAT filesystem.
    /// Any changes made to the filesystem are kept in memory, but not written back to the file.
    fn open_img_gz(path: &str) -> FatFS {
        let file = File::open(path).unwrap();
        let mut gz_decoder = flate2::read::GzDecoder::new(file);
        let mut buf = vec![];
        gz_decoder.read_to_end(&mut buf).unwrap();
        FatFS::new(block_from_file(Cursor::new(buf)))
    }
    #[test]
    fn test() {
        println!("{:?}", std::env::current_dir());
        let _fat = open_img_gz("tests/fat16.img.gz");
    }
}
