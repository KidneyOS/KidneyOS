
use kidneyos::dev::block::BlockDriver;
use crate::structs;

struct vsfs;

impl structs::FileSystem for vsfs {
    fn new() {

    }
    fn open(&self, path: &str) -> Option<File> {

    }
    fn close(&self, file: &File) -> bool {

    }
    fn read(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32;
    fn write(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32;
    fn create(&mut self, path: &str, name: &str) -> bool;
    fn delete(&self, path: &str) -> bool;
    fn list_dir(&self, path: &str) -> Option<Vec<String>>;
    fn mkdir(&mut self, path: &str, name: &str) -> bool;
    fn rmdir(&mut self, path: &str, name: &str) -> bool;
}
    