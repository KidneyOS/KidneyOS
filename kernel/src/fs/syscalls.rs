use crate::fs::fs_manager::{Mode, ROOT};
use crate::paging::is_readable_user_address;
use crate::user_program::syscall::{EFAULT, EINVAL, ENOENT};

pub const O_CREATE: usize = 0x40;

/// # Safety
///
/// TODO: actually check path is a valid pointer then mark this as safe
pub unsafe fn open(path: *const u8, flags: usize) -> isize {
    if (flags & !O_CREATE) != 0 {
        return -EINVAL;
    }
    let mut path_len = 0usize;
    let mut valid = false;
    while is_readable_user_address(path.wrapping_add(path_len)) {
        if *path.wrapping_add(path_len) == 0 {
            valid = true;
            break;
        }
        path_len += 1;
    }
    if !valid {
        return -EFAULT;
    }
    let path = core::slice::from_raw_parts(path, path_len);
    let Ok(path) = core::str::from_utf8(path) else {
        // invalid UTF-8 in path
        return -ENOENT;
    };
    let mode = if (flags & O_CREATE) != 0 {
        Mode::CreateReadWrite
    } else {
        Mode::ReadWrite
    };
    // TODO: is this safe? check w process team
    //       if so, we should add a convenience function, getpid()
    let pid = crate::threading::RUNNING_THREAD.as_ref().unwrap().pid;
    match ROOT.lock().open(path, pid, mode) {
        Err(e) => e.to_isize(),
        Ok(fd) => fd.into(),
    }
}
