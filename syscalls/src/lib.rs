#![no_std]

use core::arch::asm;

type Pid = u16;

#[repr(C)]
pub struct Timespec {
    // ... unsized ...
    // TODO: Fill for nanosleep.
}

mod defs;
pub use defs::*;

#[no_mangle]
pub extern "C" fn exit(code: usize) {
    unsafe {
        asm!("
            mov eax, 0x1
            int 0x80
        ", in("ebx") code);
    }
}

#[no_mangle]
pub extern "C" fn fork() {
    unsafe {
        asm!(
            "
            mov eax, 0x2
            int 0x80
        "
        );
    }
}

#[no_mangle]
pub extern "C" fn read(fd: i32, buffer: *mut u8, count: usize) -> i32 {
    let result;
    unsafe {
        asm!("
            mov eax, 0x3
            int 0x80
        ", in("ebx") fd, in("ecx") buffer, in("edx") count, out("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn write(fd: i32, buffer: *const u8, count: usize) -> i32 {
    let result;
    unsafe {
        asm!("
            mov eax, 0x4
            int 0x80
        ", in("ebx") fd, in("ecx") buffer, in("edx") count, out("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn open(name: *const u8, flags: usize) -> i32 {
    let result;
    unsafe {
        asm!("
            mov eax, 0x5
            int 0x80
        ", in("ebx") name, in("ecx") flags, out("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn close(fd: i32) -> i32 {
    let result;
    unsafe {
        asm!("
            mov eax, 0x6
            int 0x80
        ", in("ebx") fd, out("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn lseek64(fd: i32, offset: i64, whence: i32) -> i64 {
    let mut offset = offset;
    let result: i32;
    unsafe {
        asm!("
            mov eax, 0x8c
            int 0x80
        ", in("ebx") fd, in("ecx") (core::ptr::addr_of_mut!(offset)), in("edx") whence, out("eax") result);
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
            mov eax, 0xb7
            int 0x80
        ", in("ebx") buf, in("ecx") size, out("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn chdir(path: *const i8) -> i32 {
    let result;
    unsafe {
        asm!("
            mov eax, 0xc
            int 0x80
        ", in("ebx") path, out("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn mkdir(path: *const i8) -> i32 {
    let result;
    unsafe {
        asm!("
            mov eax, 0x27
            int 0x80
        ", in("ebx") path, out("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn fstat(fd: i32, statbuf: *mut Stat) -> i32 {
    let result;
    unsafe {
        asm!("
            mov eax, 0x6c
            int 0x80
        ", in("ebx") fd, in("ecx") statbuf, out("eax") result);
    }
    result
}

#[no_mangle]
pub extern "C" fn waitpid(pid: Pid, stat: *mut i32, options: i32) {
    unsafe {
        asm!("
            mov eax, 0x8c
            int 0x80
        ", in("ebx") pid, in("ecx") stat, in("edx") options);
    }
}

#[no_mangle]
pub extern "C" fn execve(filename: *const i8, argv: *const *const i8, envp: *const *const i8) {
    unsafe {
        asm!("
            mov eax, 0x7
            int 0x80
        ", in("ebx") filename, in("ecx") argv, in("edx") envp);
    }
}

// Seems to reference __kernel_timespec as the inputs for this syscall.
// Not sure if we have this implemented.
#[no_mangle]
pub extern "C" fn nanosleep(duration: *const Timespec, remainder: *mut Timespec) {
    unsafe {
        asm!("
            mov eax, 0xA2
            int 0x80
        ", in("ebx") duration, in("ecx") remainder);
    }
}

#[no_mangle]
pub extern "C" fn scheduler_yield() {
    unsafe {
        asm!(
            "
            mov eax, 0x9E
            int 0x80
        "
        );
    }
}
