// syscall constants and types
// These are in a separate file so that both the kernel code and userspace libc can include/use them.

#[repr(C)]
pub struct Stat {
    pub inode: u32,
    pub nlink: u32,
    pub size: u64,
    pub r#type: u8,
}

#[repr(C)]
pub struct Dirent {
    /// Opaque offset value to be used with seekdir.
    pub offset: u64,
    pub inode: u32,
    /// Length of this directory entry in bytes.
    pub reclen: u16,
    pub r#type: u8,
    /// Null-terminated file name
    pub name: [u8; 0],
}

pub const O_CREATE: usize = 0x40;

pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;

pub const ENOENT: isize = 2;
pub const EIO: isize = 5;
pub const EBADF: isize = 9;
pub const EFAULT: isize = 14;
pub const EBUSY: isize = 16;
pub const EEXIST: isize = 17;
pub const EXDEV: isize = 18;
pub const ENODEV: isize = 19;
pub const ENOTDIR: isize = 20;
pub const EISDIR: isize = 21;
pub const EINVAL: isize = 22;
pub const EMFILE: isize = 24;
pub const ENOSPC: isize = 28;
pub const ESPIPE: isize = 29;
pub const EROFS: isize = 30;
pub const EMLINK: isize = 31;
pub const ERANGE: isize = 34;
pub const ENOSYS: isize = 38;
pub const ENOTEMPTY: isize = 39;
pub const ELOOP: isize = 40;

pub const SYS_EXIT: usize = 0x1;
pub const SYS_FORK: usize = 0x2;
pub const SYS_READ: usize = 0x3;
pub const SYS_WRITE: usize = 0x4;
pub const SYS_OPEN: usize = 0x5;
pub const SYS_CLOSE: usize = 0x6;
pub const SYS_WAITPID: usize = 0x7;
pub const SYS_LINK: usize = 0x9;
pub const SYS_UNLINK: usize = 0x0a;
pub const SYS_EXECVE: usize = 0x0b;
pub const SYS_CHDIR: usize = 0xc;
pub const SYS_MOUNT: usize = 0x15;
pub const SYS_UNMOUNT: usize = 0x16;
pub const SYS_SYNC: usize = 0x24;
pub const SYS_RENAME: usize = 0x26;
pub const SYS_MKDIR: usize = 0x27;
pub const SYS_RMDIR: usize = 0x28;
pub const SYS_SYMLINK: usize = 0x53;
pub const SYS_FTRUNCATE: usize = 0x5d;
pub const SYS_FSTAT: usize = 0x6c;
pub const SYS_LSEEK64: usize = 0x8c;
pub const SYS_GETDENTS: usize = 0x8d;
pub const SYS_NANOSLEEP: usize = 0xa2;
pub const SYS_SCHED_YIELD: usize = 0x9e;
pub const SYS_GETCWD: usize = 0xb7;

pub const S_REGULAR_FILE: u8 = 1;
pub const S_SYMLINK: u8 = 2;
pub const S_DIRECTORY: u8 = 3;
