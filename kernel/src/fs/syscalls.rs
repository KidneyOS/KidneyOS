use crate::fs::{
    fs_manager::{Mode, SeekFrom, ROOT},
    FileDescriptor, ProcessFileDescriptor,
};
use crate::mem::util::{
    get_cstr_from_user_space, get_mut_from_user_space, get_mut_slice_from_user_space,
    get_slice_from_user_space, CStrError,
};
use crate::threading::{
    process_table::PROCESS_TABLE, thread_control_block::ProcessControlBlock, RUNNING_THREAD,
};
use crate::user_program::syscall::{
    Dirent, Stat, EBADF, EFAULT, EINVAL, ENOENT, ERANGE, O_CREATE, SEEK_CUR, SEEK_END, SEEK_SET,
};

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

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_cstr_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn open(path: *const u8, flags: usize) -> isize {
    if (flags & !O_CREATE) != 0 {
        return -EINVAL;
    }
    let path = match get_cstr_from_user_space(path) {
        Ok(s) => s,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let mode = if (flags & O_CREATE) != 0 {
        Mode::CreateReadWrite
    } else {
        Mode::ReadWrite
    };
    match ROOT.lock().open(running_process(), path, mode) {
        Err(e) => -e.to_isize(),
        Ok(fd) => fd.into(),
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_mut_slice_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn read(fd: usize, buf: *mut u8, count: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    // do reads of at most 128KB to not starve other processes
    let count = core::cmp::min(count, 128 << 10);
    let Some(buf) = get_mut_slice_from_user_space::<u8>(buf, count) else {
        return -EFAULT;
    };
    let fd = ProcessFileDescriptor {
        pid: running_process().pid,
        fd,
    };
    match ROOT.lock().read(fd, buf) {
        Err(e) => -e.to_isize(),
        Ok(n) => n as isize,
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_mut_slice_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn write(fd: usize, buf: *const u8, count: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    // do writes of at most 128KB to not starve other processes
    let count = core::cmp::min(count, 128 << 10);
    let Some(buf) = get_slice_from_user_space::<u8>(buf, count) else {
        return -EFAULT;
    };
    let fd = ProcessFileDescriptor {
        pid: running_process().pid,
        fd,
    };
    match ROOT.lock().write(fd, buf) {
        Err(e) => -e.to_isize(),
        Ok(n) => n as isize,
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_mut_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn lseek64(fd: usize, offset: *mut i64, whence: isize) -> isize {
    let Some(offset) = get_mut_from_user_space(offset) else {
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
        pid: unsafe { running_process().pid },
        fd,
    };
    match ROOT.lock().lseek(fd, whence, *offset) {
        Err(e) => -e.to_isize(),
        Ok(n) => {
            *offset = n;
            0
        }
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when accessing running PCB is safe
pub unsafe fn close(fd: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    let fd = ProcessFileDescriptor {
        pid: running_process().pid,
        fd,
    };
    match ROOT.lock().close(fd) {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_cstr_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn chdir(path: *const u8) -> isize {
    let path = match get_cstr_from_user_space(path) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match ROOT.lock().chdir(running_process_mut(), path) {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_mut_slice_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn getcwd(buf: *mut u8, size: usize) -> isize {
    let Some(buf) = get_mut_slice_from_user_space(buf, size) else {
        return -EFAULT;
    };
    let pcb = running_process();
    let cwd = pcb.cwd_path.as_bytes();
    if size < cwd.len() + 1 {
        return -ERANGE;
    }
    buf[..cwd.len()].copy_from_slice(cwd);
    buf[cwd.len()] = 0;
    0
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_cstr_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn mkdir(path: *const u8) -> isize {
    let path = match get_cstr_from_user_space(path) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match ROOT.lock().mkdir(running_process(), path) {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_mut_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn fstat(fd: usize, statbuf: *mut Stat) -> isize {
    let Some(statbuf) = get_mut_from_user_space(statbuf) else {
        return -EFAULT;
    };
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    let fd = ProcessFileDescriptor {
        pid: unsafe { running_process().pid },
        fd,
    };
    match ROOT.lock().fstat(fd) {
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

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_cstr_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn unlink(path: *const u8) -> isize {
    let path = match get_cstr_from_user_space(path) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match ROOT.lock().unlink(running_process(), path) {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_cstr_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn rmdir(path: *const u8) -> isize {
    let path = match get_cstr_from_user_space(path) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match ROOT.lock().rmdir(running_process(), path) {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

pub fn sync() -> isize {
    match ROOT.lock().sync() {
        Err(e) => -e.to_isize(),
        Ok(()) => 0,
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_mut_slice_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn getdents(fd: usize, output: *mut Dirent, size: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    if get_mut_slice_from_user_space(output as *mut u8, size).is_none() {
        return -EFAULT;
    }
    let fd = ProcessFileDescriptor {
        pid: running_process().pid,
        fd,
    };
    match ROOT.lock().getdents(fd, output, size) {
        Ok(n) => n as isize,
        Err(e) => -e.to_isize(),
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_cstr_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn link(source: *const u8, dest: *const u8) -> isize {
    let source = match get_cstr_from_user_space(source) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let dest = match get_cstr_from_user_space(dest) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match ROOT.lock().link(running_process(), source, dest) {
        Ok(()) => 0,
        Err(e) => -e.to_isize(),
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_cstr_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn symlink(source: *const u8, dest: *const u8) -> isize {
    let source = match get_cstr_from_user_space(source) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let dest = match get_cstr_from_user_space(dest) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match ROOT.lock().symlink(running_process(), source, dest) {
        Ok(()) => 0,
        Err(e) => -e.to_isize(),
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_cstr_from_user_space works correctly and accessing running PCB is safe
pub unsafe fn rename(source: *const u8, dest: *const u8) -> isize {
    let source = match get_cstr_from_user_space(source) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -ENOENT,
        Err(CStrError::Fault) => return -EFAULT,
    };
    let dest = match get_cstr_from_user_space(dest) {
        Ok(path) => path,
        Err(CStrError::BadUtf8) => return -EINVAL,
        Err(CStrError::Fault) => return -EFAULT,
    };
    match ROOT.lock().rename(running_process(), source, dest) {
        Ok(()) => 0,
        Err(e) => -e.to_isize(),
    }
}

// TODO: mount, unmount, ftruncate
