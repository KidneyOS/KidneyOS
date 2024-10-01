pub mod fs_manager;
use crate::threading::thread_control_block::Pid;

pub type FileDescriptor = i16;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessFileDescriptor {
    pub pid: Pid,
    pub fd: FileDescriptor,
}
