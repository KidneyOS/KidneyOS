pub mod fs_manager;
pub mod syscalls;
use crate::fs::fs_manager::Mode;
use crate::threading::{
    process_table::PROCESS_TABLE,
    thread_control_block::{Pid, ProcessControlBlock},
    RUNNING_THREAD,
};
use crate::vfs::{Path, Result};
use alloc::{vec, vec::Vec};

pub type FileDescriptor = i16;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessFileDescriptor {
    pub pid: Pid,
    pub fd: FileDescriptor,
}

unsafe fn running_process() -> &'static ProcessControlBlock {
    PROCESS_TABLE
        .as_ref()
        .unwrap()
        .get(RUNNING_THREAD.as_ref().unwrap().as_ref().pid)
        .unwrap()
}

unsafe fn running_process_mut() -> &'static mut ProcessControlBlock {
    PROCESS_TABLE
        .as_mut()
        .unwrap()
        .get_mut(RUNNING_THREAD.as_ref().unwrap().as_ref().pid)
        .unwrap()
}

/// Read entire contents of file to kernel memory.
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    // technically UB if other mut references to RUNNING_THREAD exist. we should really use a mutex for RUNNING_THREAD…
    let process = unsafe { running_process() };
    let mut root = fs_manager::ROOT.lock();
    let fd = root.open(process, path, Mode::ReadWrite)?;
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
