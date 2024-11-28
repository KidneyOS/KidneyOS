// Ordinarily, a function dereferencing a raw pointer argument almost always requires it to be unsafe.
// Here we should be fine since we are checking the validity of pointers.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::fs::{
    fs_manager::{Mode, SeekFrom},
    FileDescriptor, ProcessFileDescriptor,
};
use crate::fs::fs_manager::RootFileSystem;
use crate::mem::util::{
    get_cstr_from_user_space, get_mut_from_user_space, get_mut_slice_from_user_space,
    get_slice_from_user_space, CStrError,
};
use crate::system::{root_filesystem, running_process, running_thread_pid};
use crate::user_program::syscall::{
    Dirent, Stat, EBADF, EFAULT, EINVAL, ENODEV, ENOENT, ERANGE, O_CREATE, SEEK_CUR, SEEK_END,
    SEEK_SET,
};
use crate::vfs::tempfs::TempFS;

pub fn open(path: *const u8, flags: usize) -> isize {
    if (flags & !O_CREATE) != 0 {
        return -EINVAL;
    }
    let path = match unsafe { get_cstr_from_user_space(path) } {
        Ok(s) => s,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let mode = if (flags & O_CREATE) != 0 {
        Mode::CreateReadWrite
    } else {
        Mode::ReadWrite
    };
    match root_filesystem()
        .lock()
        .open(&running_process().lock(), path, mode)
    {
        Err(e) => -e.to_isize(),
        Ok(fd) => fd.into(),
    }
}

pub fn read(fd: usize, buf: *mut u8, count: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    // do reads of at most 128KB to not starve other processes
    let count = core::cmp::min(count, 128 << 10);
    let Some(buf) = (unsafe { get_mut_slice_from_user_space::<u8>(buf, count) }) else {
        return -EFAULT;
    };
    let fd = ProcessFileDescriptor {
        pid: running_thread_pid(),
        fd,
    };
    match RootFileSystem::read(root_filesystem(), fd, buf) {
        Err(e) => -e.to_isize(),
        Ok(n) => n as isize,
    }
}

pub fn write(fd: usize, buf: *const u8, count: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    // do writes of at most 128KB to not starve other processes
    let count = core::cmp::min(count, 128 << 10);
    let Some(buf) = (unsafe { get_slice_from_user_space::<u8>(buf, count) }) else {
        return -EFAULT;
    };
    let fd = ProcessFileDescriptor {
        pid: running_thread_pid(),
        fd,
    };
    match RootFileSystem::write(root_filesystem(), fd, buf) {
        Err(e) => -e.to_isize(),
        Ok(n) => n as isize,
    }
}

pub fn lseek64(fd: usize, offset: *mut i64, whence: isize) -> isize {
    let Some(offset) = (unsafe { get_mut_from_user_space(offset) }) else {
        return -EFAULT;
    };
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    let whence = match whence as i32 {
        SEEK_SET => SeekFrom::Start,
        SEEK_CUR => SeekFrom::Current,
        SEEK_END => SeekFrom::End,
        _ => return -EINVAL,
    };
    let fd = ProcessFileDescriptor {
        pid: running_thread_pid(),
        fd,
    };
    match root_filesystem().lock().lseek(fd, whence, *offset) {
        Err(e) => -e.to_isize(),
        Ok(n) => {
            *offset = n;
            0
        }
    }
}

pub fn close(fd: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    let fd = ProcessFileDescriptor {
        pid: running_thread_pid(),
        fd,
    };
    match root_filesystem().lock().close(fd) {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

pub fn chdir(path: *const u8) -> isize {
    let path = match unsafe { get_cstr_from_user_space(path) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match root_filesystem()
        .lock()
        .chdir(&mut running_process().lock(), path)
    {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

pub fn getcwd(buf: *mut u8, size: usize) -> isize {
    let Some(buf) = (unsafe { get_mut_slice_from_user_space(buf, size) }) else {
        return -EFAULT;
    };
    let pcb = running_process();
    let pcb = pcb.lock();
    let cwd = pcb.cwd_path.as_bytes();
    if size < cwd.len() + 1 {
        return -ERANGE;
    }
    buf[..cwd.len()].copy_from_slice(cwd);
    buf[cwd.len()] = 0;
    0
}

pub fn mkdir(path: *const u8) -> isize {
    let path = match unsafe { get_cstr_from_user_space(path) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match root_filesystem()
        .lock()
        .mkdir(&running_process().lock(), path)
    {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

pub fn fstat(fd: usize, statbuf: *mut Stat) -> isize {
    let Some(statbuf) = (unsafe { get_mut_from_user_space(statbuf) }) else {
        return -EFAULT;
    };
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    let fd = ProcessFileDescriptor {
        pid: running_thread_pid(),
        fd,
    };
    match root_filesystem().lock().fstat(fd) {
        Err(e) => -e.to_isize(),
        Ok(info) => {
            *statbuf = Stat {
                inode: info.inode,
                size: info.size,
                nlink: info.nlink,
                r#type: info.r#type.to_u8(),
            };
            0
        }
    }
}

pub fn unlink(path: *const u8) -> isize {
    let path = match unsafe { get_cstr_from_user_space(path) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match root_filesystem()
        .lock()
        .unlink(&running_process().lock(), path)
    {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

pub fn rmdir(path: *const u8) -> isize {
    let path = match unsafe { get_cstr_from_user_space(path) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match root_filesystem()
        .lock()
        .rmdir(&running_process().lock(), path)
    {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

pub fn sync() -> isize {
    match root_filesystem().lock().sync() {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

pub fn getdents(fd: usize, output: *mut Dirent, size: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    if unsafe { get_mut_slice_from_user_space(output as *mut u8, size) }.is_none() {
        return -EFAULT;
    }
    let fd = ProcessFileDescriptor {
        pid: running_thread_pid(),
        fd,
    };
    // SAFETY: just checked that output..output + size is valid above
    match unsafe { root_filesystem().lock().getdents(fd, output, size) } {
        Ok(n) => n as isize,
        Err(e) => -e.to_isize(),
    }
}

pub fn link(source: *const u8, dest: *const u8) -> isize {
    let source = match unsafe { get_cstr_from_user_space(source) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let dest = match unsafe { get_cstr_from_user_space(dest) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match root_filesystem()
        .lock()
        .link(&running_process().lock(), source, dest)
    {
        Ok(()) => 0,
        Err(e) => -e.to_isize(),
    }
}

pub fn symlink(source: *const u8, dest: *const u8) -> isize {
    let source = match unsafe { get_cstr_from_user_space(source) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let dest = match unsafe { get_cstr_from_user_space(dest) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match root_filesystem()
        .lock()
        .symlink(&running_process().lock(), source, dest)
    {
        Ok(()) => 0,
        Err(e) => -e.to_isize(),
    }
}

pub fn rename(source: *const u8, dest: *const u8) -> isize {
    let source = match unsafe { get_cstr_from_user_space(source) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let dest = match unsafe { get_cstr_from_user_space(dest) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match root_filesystem()
        .lock()
        .rename(&running_process().lock(), source, dest)
    {
        Ok(()) => 0,
        Err(e) => -e.to_isize(),
    }
}

pub fn ftruncate(fd: usize, size_lo: usize, size_hi: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    let fd = ProcessFileDescriptor {
        pid: running_thread_pid(),
        fd,
    };
    let size = size_lo as u64 | (size_hi as u64) << 32;
    match root_filesystem().lock().ftruncate(fd, size) {
        Ok(()) => 0,
        Err(e) => -e.to_isize(),
    }
}

pub fn unmount(path: *const u8) -> isize {
    let path = match unsafe { get_cstr_from_user_space(path) } {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match root_filesystem()
        .lock()
        .unmount(&running_process().lock(), path)
    {
        Ok(()) => 0,
        Err(e) => -e.to_isize(),
    }
}

pub fn mount(device: *const u8, target: *const u8, file_system_type: *const u8) -> isize {
    let device = match unsafe { get_cstr_from_user_space(device) } {
        Ok(d) => d,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let target = match unsafe { get_cstr_from_user_space(target) } {
        Ok(d) => d,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let file_system_type = match unsafe { get_cstr_from_user_space(file_system_type) } {
        Ok(d) => d,
        Err(CStrError::BadUtf8) => return -ENODEV,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let mut root = root_filesystem().lock();
    let result = match file_system_type {
        "tmpfs" => {
            if !device.is_empty() {
                // should set device to empty string for tmpfs
                return -EINVAL;
            }
            root.mount(&running_process().lock(), target, TempFS::new())
        }
        _ => return -ENODEV,
    };
    match result {
        Ok(()) => 0,
        Err(e) => -e.to_isize(),
    }
}

pub fn dup(fd: isize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    
    let pid = running_process().lock().pid;
    
    let process_fd = ProcessFileDescriptor {
        pid,
        fd
    };
    
    root_filesystem().lock().dup(pid, process_fd)
        .map(|i| i.into())
        .unwrap_or_else(|err| -err.to_isize())
}

pub fn dup2(old: isize, new: isize) -> isize {
    let Ok(old) = FileDescriptor::try_from(old) else {
        return -EBADF;
    };

    let Ok(new) = FileDescriptor::try_from(new) else {
        return -EBADF;
    };

    let pid = running_process().lock().pid;

    let old_process_fd = ProcessFileDescriptor {
        pid,
        fd: old,
    };
    
    let new_process_fd = ProcessFileDescriptor {
        pid,
        fd: new,
    };
    
    root_filesystem().lock().dup2(old_process_fd, new_process_fd)
        .map(|_| 0)
        .unwrap_or_else(|err| -err.to_isize())
}

pub fn pipe(fds: *mut isize) -> isize {
    let Some(fds) = (unsafe { get_mut_slice_from_user_space(fds, 2) }) else {
        return -EFAULT
    };
    
    let pid = running_process().lock().pid;
    
    match root_filesystem().lock().pipe(pid) {
        Ok((read_end, write_end)) => {
            fds[0] = read_end as isize;
            fds[1] = write_end as isize;
            
            0
        }
        Err(e) => -e.to_isize()
    }
}
