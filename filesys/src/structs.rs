
use crate::disk_device::Disk;
use crate::fat;

enum DiskInode {
    Fat16Inode(fat::FSEntry)
}

enum FileSystemType<'a> {
    Fat16(fat::Fat16<'a>),
    // vsfs(vsfs::Vsfs<'a>),
}

pub struct File {
    mem_inode: mem_inode,
    pathname: String
}
// new code
// direntry
pub struct mem_inode<'a>{
    fs: FileSystemType,
    disk_inode: DiskInode,
}

struct MountPoint {
    path: String,
    fs: FileSystemType
}

pub struct VirtualFileSystem {
    root_fs: FileSystemType, // maps onto inode 0 in rootfs, should probably be chosen by kernel i think should point to ide channel 0 drive 0
    mount_points: Vec<MountPoint>,
}

impl VirtualFileSystem {
    pub fn new(disk: &Disk) -> Self {
        VirtualFileSystem {
            root_fs: FileSystemType::Fat16(fat::Fat16::new(disk: &Disk).unwrap()),
            mount_points: Vec::new(),
        }
    }
    fn resolve_path(&self, global_path: &str) -> (FileSystemType, String) {
        let mut longest_match = None;
        for mount in self.mount_points.iter() {
            if global_path.starts_with(&mount.path) {
                match longest_match {
                    Some(mut longest_match) => {
                        if mount.path.len() > longest_match.len() {
                            longest_match = Some(mount);
                        }
                    }
                    None => {
                        longest_match = Some(mount);
                    }
                }

            }
        }

        match longest_match {
            Some(longest_match) => {
                let mount = longest_match.unwrap();
                let inner_path = global_path.replace(&mount.path, "");
                (&mount.fs, inner_path)
            }
            None => {
                (&self.root_fs, global_path)
            }
        }

    }


    fn lookup(&self, global)




    pub fn mount(&mut self, path: String, fs: FileSystemType) {
        if self.mount_points.iter().any(|mount| mount.path == path) {
            return;
        }

        let mount_point = MountPoint {
           path,
           fs
        };

        self.mount_points.push(mount_point);
    }

    pub fn unmount(&mut self, path: &str) {
        let index = self.mount_points.iter().position(|mount| mount.path == path);

        match index {
            Some(index) => {
                self.mount_points.remove(index);
            }
            None => {}
        }
    }

    // file operations
    pub fn open(&self, path: &str) -> Result<File, io::Error>{
        let (fs, inner_path) = self.resolve_path(path)?;
        fs.open(&inner_path)
    }
    pub fn close(&self, file: &File) -> Result<(), io::Error>{
        let (fs, inner_path) = self.resolve_path(&file.pathname)?;
        fs.close(file)
    }
    pub fn read(&self, file: &File, buffer: &mut [u8], amount: u32) -> Result<u32, io::Error>{
        let (fs, _) = self.resolve_path(&file.pathname)?;
        fs.read(file, buffer, amount)
    }
    pub fn write(&self, file: &File, buffer: &mut [u8], amount: u32) -> Result<u32, io::Error>{
        let (fs, _) = self.resolve_path(&file.pathname)?;
        fs.write(file, buffer, amount)
    }

    // directory operations
    pub fn create(&self, path: &str, name: &str) -> Result<(), io::Error>{
        let (fs, inner_path) = self.resolve_path(path)?;
        fs.create(inner_path, name)
    }
    pub fn delete(&self, path: &str) -> Result<(), io::Error>{
        let (fs, inner_path) = self.resolve_path(path)?;
        fs.delete(inner_path)
    }
    pub fn list_dir(&self, path: &str) -> Result<Vec<String>, io::Error>{
        let (fs, inner_path) = self.resolve_path(path)?;
        fs.list_dir(inner_path)
    }
    pub fn mkdir(&mut self, path: &str, name: &str) -> Result<(), io::Error>{
        let (fs, inner_path) = self.resolve_path(path)?;
        fs.mkdir(inner_path, name)
    }
    pub fn rmdir(&mut self, path: &str, name: &str) -> Result<(), io::Error>{
        let (fs, inner_path) = self.resolve_path(path)?;
        fs.rmdir(inner_path, name)
    }
}

pub trait FileSystem {
    fn new();
    // fn mount(&mut self);
    // fn unmount(&mut self);
    fn open(&self, path: &str) -> Option<File>;
    fn close(&self, file: &File) -> bool;
    fn read(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32;
    fn write(&self, file: &File, buffer: &mut [u8], amount: u32) -> u32;
    fn create(&mut self, path: &str, name: &str) -> bool;
    fn delete(&self, path: &str) -> bool;
    fn list_dir(&self, path: &str) -> Option<Vec<String>>;
    fn mkdir(&mut self, path: &str, name: &str) -> bool;
    fn rmdir(&mut self, path: &str, name: &str) -> bool;
}
