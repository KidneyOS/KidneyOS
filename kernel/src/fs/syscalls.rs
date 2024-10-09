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
use crate::user_program::syscall::{EBADF, EFAULT, EINVAL, ENOENT, ERANGE};

pub const O_CREATE: usize = 0x40;

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

pub const SEEK_SET: isize = 0;
pub const SEEK_CUR: isize = 1;
pub const SEEK_END: isize = 2;

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
    let whence = match whence {
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
