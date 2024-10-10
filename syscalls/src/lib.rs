#![no_std]

use core::arch::asm;

type Pid = u16;

#[repr(C)]
pub struct Timespec {
    // ... unsized ...
    // TODO: Fill for nanosleep.
}

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
pub extern "C" fn read(fd: u32, buffer: *mut u8, count: usize) {
    unsafe {
        asm!("
            mov eax, 0x3
            int 0x80
        ", in("ebx") fd, in("ecx") buffer, in("edx") count);
    }
}

#[no_mangle]
pub extern "C" fn waitpid(pid: Pid, stat: *mut i32, options: i32) {
    unsafe {
        asm!("
            mov eax, 0x7
            int 0x80
        ", in("ebx") pid, in("ecx") stat, in("edx") options);
    }
}

// Temporarily defining execve as (elf_bytes: *const u8, count: usize) while FS comes together.
/*
#[no_mangle]
pub extern "C" fn execve(filename: *const i8, argv: *const *const i8, envp: *const *const i8) {
    unsafe {
        asm!("
            mov eax, 0x0b
            int 0x80
        ", in("ebx") filename, in("ecx") argv, in("edx") envp);
    }
}
*/

#[no_mangle]
pub extern "C" fn execve(elf_bytes: *const u8, byte_count: usize) {
    unsafe {
        asm!("
            mov eax, 0x0b
            int 0x80
        ", in("ebx") elf_bytes, in("ecx") byte_count)
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
