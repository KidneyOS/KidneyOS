#![no_std]

use core::arch::asm;
use core::ffi::c_char;

pub type Pid = u16;

#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

pub mod defs;
pub use defs::*;

#[no_mangle]
pub extern "C" fn exit(code: i32) {
    unsafe {
        asm!(
            "
            mov eax, 0x1
            int 0x80
            ",
            in("ebx") code,
        );
    }
}

#[allow(clippy::cast_possible_truncation)]
#[no_mangle]
pub extern "C" fn fork() -> Pid {
    let result: i32;
    unsafe {
        asm!(
            "
            mov eax, 0x2
            int 0x80
            ", 
            lateout("eax") result,
        );
    }
    result as Pid
}

#[no_mangle]
pub extern "C" fn read(fd: i32, buffer: *mut u8, count: usize) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_READ, in("ebx") fd, in("ecx") buffer, in("edx") count, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn write(fd: i32, buffer: *const u8, count: usize) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_WRITE, in("ebx") fd, in("ecx") buffer, in("edx") count, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn open(name: *const c_char, flags: usize) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_OPEN, in("ebx") name, in("ecx") flags, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn close(fd: i32) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_CLOSE, in("ebx") fd, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn lseek64(fd: i32, offset: i64, whence: i32) -> i64 {
    let mut offset = offset;
    let result: i32;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_LSEEK64,
            in("ebx") fd, in("ecx") core::ptr::addr_of_mut!(offset),
            in("edx") whence, lateout("eax") result);
    }
    if result < 0 {
        result.into()
    } else {
        offset
    }
}

#[no_mangle]
pub extern "C" fn getcwd(buf: *mut i8, size: usize) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_GETCWD, in("ebx") buf, in("ecx") size, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn chdir(path: *const c_char) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_CHDIR, in("ebx") path, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn mkdir(path: *const c_char) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_MKDIR, in("ebx") path, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn fstat(fd: i32, statbuf: *mut Stat) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_FSTAT, in("ebx") fd, in("ecx") statbuf, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn unlink(path: *const c_char) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_UNLINK, in("ebx") path, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn link(source: *const c_char, dest: *const c_char) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_LINK, in("ebx") source, in("ecx") dest, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn symlink(source: *const c_char, dest: *const c_char) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_SYMLINK, in("ebx") source, in("ecx") dest, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn rename(source: *const c_char, dest: *const c_char) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_RENAME, in("ebx") source, in("ecx") dest, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn rmdir(path: *const c_char) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_RMDIR, in("ebx") path, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn getdents(fd: i32, output: *mut Dirent, size: usize) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_GETDENTS, in("ebx") fd, in("ecx") output, in("edx") size, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn ftruncate(fd: i32, size: u64) -> i32 {
    let result;
    #[allow(clippy::cast_possible_truncation)]
    let size_lo = size as u32;
    let size_hi = (size >> 32) as u32;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_FTRUNCATE, in("ebx") fd, in("ecx") size_lo, in("edx") size_hi, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn sync() -> i32 {
    let result;
    unsafe {
        asm!("int 0x80", in("eax") SYS_SYNC, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn unmount(path: *const c_char) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_UNMOUNT, in("ebx") path, lateout("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn mount(
    device: *const c_char,
    target: *const c_char,
    filesystem_type: *const c_char,
) -> i32 {
    let result;
    unsafe {
        asm!("
            int 0x80
        ", in("eax") SYS_MOUNT, in("ebx") device, in("ecx") target, in("edx") filesystem_type, lateout("eax") result);
    }
    result
}

#[no_mangle]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn waitpid(pid: Pid, stat: *mut i32, options: i32) -> Pid {
    let result: i32;
    unsafe {
        asm!("
            mov eax, 0x7
            int 0x80
            ", 
            in("ebx") pid,
            in("ecx") stat,
            in("edx") options,
            lateout("eax") result,
        );
    }
    result as Pid
}

#[no_mangle]
pub extern "C" fn dup(fd: i32) -> i32 {
    let result: i32;

    unsafe {
        asm!(
            "int 0x80",
            in("eax") SYS_DUP,
            in("ebx") fd,
            lateout("eax") result,
        );
    }

    result
}

#[no_mangle]
pub extern "C" fn dup2(old_fd: i32, new_fd: i32) -> i32 {
    let result: i32;

    unsafe {
        asm!(
            "int 0x80",
            in("eax") SYS_DUP2,
            in("ebx") old_fd,
            in("ecx") new_fd,
            lateout("eax") result,
        );
    }

    result
}
#[no_mangle]
pub extern "C" fn pipe(fds: *mut i32) -> i32 {
    let result: i32;
    
    unsafe {
        asm!(
            "int 0x80",
            in("eax") SYS_PIPE,
            in("ebx") fds,
            lateout("eax") result,
        );
    }
    
    result
}

#[no_mangle]
pub extern "C" fn execve(
    filename: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> i32 {
    let result: i32;

    unsafe {
        asm!(
            "int 0x80",
            in("eax") SYS_EXECVE,
            in("ebx") filename,
            in("ecx") argv,
            in("edx") envp,
            lateout("eax") result
        )
    }

    result
}

// Seems to reference __kernel_timespec as the inputs for this syscall.
// Not sure if we have this implemented.
#[no_mangle]
pub extern "C" fn nanosleep(duration: *const Timespec, remainder: *mut Timespec) -> i32 {
    let result: i32;
    unsafe {
        asm!(
            "
            mov eax, 0xA2
            int 0x80
            ", 
            in("ebx") duration,
            in("ecx") remainder,
            lateout("eax") result,
        );
    }
    result
}

#[no_mangle]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn getpid() -> Pid {
    let result: i32;
    unsafe {
        asm!(
            "
            mov eax, 0x14
            int 0x80
            ",
            lateout("eax") result
        )
    }
    result as Pid
}

#[no_mangle]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn getppid() -> Pid {
    let result: i32;
    unsafe {
        asm!(
            "
            mov eax, 0x40
            int 0x80
            ",
            lateout("eax") result
        )
    }
    result as Pid
}

#[no_mangle]
pub extern "C" fn scheduler_yield() -> i32 {
    let result: i32;
    unsafe {
        asm!(
            "
            mov eax, 0x9E
            int 0x80
            ", 
            lateout("eax") result,
        );
    }
    result
}

#[no_mangle]
pub extern "C" fn clock_gettime(clock_id: i32, timespec: *mut Timespec) -> i32 {
    let result: i32;
    unsafe {
        asm!(
            "
            mov eax, 0x109
            int 0x80
            ",
            in("ebx") clock_id,
            in("ecx") timespec,
            lateout("eax") result,
        )
    }
    result
}

#[no_mangle]
pub extern "C" fn getrandom(buf: *mut i8, size: usize, flags: usize) -> i32 {
    let result: i32;
    unsafe {
        asm!(
            "
            mov eax, 0x163
            int 0x80
            ",
            in("ebx") buf,
            in("ecx") size,
            in("edx") flags,
            lateout("eax") result,
        )
    }
    result
}
