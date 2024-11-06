pub mod fat;
pub mod fs_manager;
pub mod syscalls;
use crate::fs::fs_manager::Mode;
use crate::system::running_process;
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
    let process = running_process();
    let mut root = fs_manager::ROOT.lock();
    let fd = root.open(&process, path, Mode::ReadWrite)?;
    let fd = ProcessFileDescriptor {
        fd,
        pid: process.pid,
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
