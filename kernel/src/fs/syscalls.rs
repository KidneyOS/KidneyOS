use crate::fs::{
    fs_manager::{Mode, ROOT},
    FileDescriptor, ProcessFileDescriptor,
};
use crate::mem::util::{
    get_cstr_from_user_space, get_mut_slice_from_user_space, get_slice_from_user_space, CStrError,
};
use crate::user_program::syscall::{EBADF, EFAULT, EINVAL, ENOENT};

pub const O_CREATE: usize = 0x40;

/// # Safety
///
/// There must not currently exist any mutable reference to [`crate::threading::RUNNING_THREAD`].
unsafe fn getpid() -> crate::threading::thread_control_block::Pid {
    crate::threading::RUNNING_THREAD.as_ref().unwrap().pid
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_cstr_from_user_space works correctly
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
    match ROOT.lock().open(path, getpid(), mode) {
        Err(e) => e.to_isize(),
        Ok(fd) => fd.into(),
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_mut_slice_from_user_space works correctly
pub unsafe fn read(fd: usize, buf: *mut u8, count: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    // do reads of at most 128KB to not starve other processes
    let count = core::cmp::min(count, 128 << 10);
    let Some(buf) = get_mut_slice_from_user_space::<u8>(buf, count) else {
        return -EFAULT;
    };
    let fd = ProcessFileDescriptor { pid: getpid(), fd };
    match ROOT.lock().read(fd, buf) {
        Err(e) => e.to_isize(),
        Ok(n) => n as isize,
    }
}

/// # Safety
///
/// TODO: mark this as no longer unsafe when get_mut_slice_from_user_space works correctly
pub unsafe fn write(fd: usize, buf: *const u8, count: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    // do reads of at most 128KB to not starve other processes
    let count = core::cmp::min(count, 128 << 10);
    let Some(buf) = get_slice_from_user_space::<u8>(buf, count) else {
        return -EFAULT;
    };
    let fd = ProcessFileDescriptor { pid: getpid(), fd };
    match ROOT.lock().write(fd, buf) {
        Err(e) => e.to_isize(),
        Ok(n) => n as isize,
    }
}

pub fn close(fd: usize) -> isize {
    let Ok(fd) = FileDescriptor::try_from(fd) else {
        return -EBADF;
    };
    let fd = ProcessFileDescriptor {
        pid: unsafe { getpid() },
        fd,
    };
    match ROOT.lock().close(fd) {
        Err(e) => e.to_isize(),
        Ok(()) => 0,
    }
}
