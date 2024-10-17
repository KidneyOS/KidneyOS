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
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn fork() -> Pid {
    let result: i32;
    unsafe {
        asm!(
            "
            mov eax, 0x2
            int 0x80
            mov {}, eax
        ", out(reg) result
        );
    }
    result as Pid
}

#[no_mangle]
pub extern "C" fn read(fd: u32, buffer: *mut u8, count: usize) -> usize {
    let result: usize;
    unsafe {
        asm!("
            mov eax, 0x3
            int 0x80
            mov {}, eax
        ", out(reg) result, in("ebx") fd, in("ecx") buffer, in("edx") count);
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
            mov {}, eax
        ", out(reg) result, in("ebx") pid, in("ecx") stat, in("edx") options);
    }
    result as Pid
}

#[no_mangle]
pub extern "C" fn execve(filename: *const i8, argv: *const *const i8, envp: *const *const i8) -> i32 {
    let result: i32;
    unsafe {
        asm!("
            mov eax, 0x7
            int 0x80
            mov {}, eax
        ", out(reg) result, in("ebx") filename, in("ecx") argv, in("edx") envp);
    }
    result
}

// Seems to reference __kernel_timespec as the inputs for this syscall.
// Not sure if we have this implemented.
#[no_mangle]
pub extern "C" fn nanosleep(duration: *const Timespec, remainder: *mut Timespec) -> i32 {
    let result: i32;
    unsafe {
        asm!("
            mov eax, 0xA2
            int 0x80
            mov {}, eax
        ", out(reg) result, in("ebx") duration, in("ecx") remainder);
    }
    result
}

#[no_mangle]
pub extern "C" fn scheduler_yield() -> i32 {
    let result: i32;
    unsafe {
        asm!(
            "
            mov eax, 0x9E
            int 0x80
            mov {}, eax
        ", out(reg) result
        );
    }
    result
}
