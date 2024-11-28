pub mod fat;
pub mod fs_manager;
pub mod syscalls;
use crate::fs::fs_manager::Mode;
use crate::system::{root_filesystem, running_process, running_thread_pid};
use crate::threading::process::Pid;
use crate::vfs::{Path, Result};
use alloc::{vec, vec::Vec};

pub type FileDescriptor = i16;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessFileDescriptor {
    pub pid: Pid,
    pub fd: FileDescriptor,
}

/// Read entire contents of file to kernel memory.
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    let mut root = root_filesystem().lock();
    let fd = root.open(&running_process().lock(), path, Mode::ReadWrite)?;
    let fd = ProcessFileDescriptor {
        fd,
        pid: running_thread_pid(),
    };
    let mut data = vec![];
    loop {
        let bytes_read = data.len();
        data.resize(bytes_read + 4096, 0);
        let n = root.read(fd, &mut data[bytes_read..])?;
        data.truncate(bytes_read + n);
        if n == 0 {
            break;
        }
    }
    Ok(data)
}

/// Write a file to an absolute path. This does not need to access the PCB, but that means it can't handle relative paths.
pub fn write_file_absolute_path(path: &Path, data: &[u8]) -> Result<()> {
    let mut root = root_filesystem().lock();
    let (fs, inode) = root.open_raw_file(None, path, Mode::CreateReadWrite)?;
    root.write_raw(fs, inode, 0, data)?;
    root.decrement_inode_ref_count(fs, inode);
    Ok(())
}
