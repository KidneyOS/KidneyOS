use std::io;
use std::fs::File;
use std::os::unix::prelude::FileExt;
pub trait DiskDevice {
    fn read_at(&self, buf: &mut [u8], sector: usize) -> Result<usize, io::Error>; // change error type?
}
pub struct IDE {
}
pub struct Test {
    pub file: std::fs::File,
    pub sector_size: usize,
}
pub enum Disk{
    Drive(IDE),
    Virtual(Test),
}
impl DiskDevice for Disk {
    fn read_at(&self, buf: &mut [u8], offset: usize) -> Result<usize, io::Error> {
        match self {
            Disk::Drive(IDE) => todo!(),
            Disk::Virtual(Test) => Test.read_at(buf, offset),
            
        }
    }
}
impl DiskDevice for Test{
    fn read_at(&self, buf: &mut [u8], sector: usize) -> Result<usize, std::io::Error> {
        self.file.read_at(buf, (self.sector_size * sector) as u64)
    }
}
