// pintos/src/threads/io.h

use core::arch::asm;

/// Reads and returns a byte from `port`.
pub fn inb(port: u16) -> u8 {
    let mut ret: u8;
    unsafe {
        asm!(
        "inb %dx, %al",
        out("al") ret,
        in("dx") port,
        options(att_syntax),
        );
    }
    ret
}

/// Reads `count` 16-bit (halfword) units from `port`, one after another, and stores them into `buf`.
pub unsafe fn insw(port: u16, buf: *mut u8, count: usize) {
    asm!(
    "push edi",
    "mov edi, eax",
    "rep insw",
    "pop edi",
    in("dx") port,
    in("eax") buf,
    in("ecx") count,
    );
}

/// Write byte `value` to `port`.
pub fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
        "outb %al, %dx",
        in("al") value,
        in("dx") port,
        options(att_syntax),
        );
    };
}

/// Writes to `port` each 16-bit unit (halfword) of data in the `count`-halfword buffer `buf`.
pub unsafe fn outsw(port: u16, buf: *const u8, count: usize) {
    asm!(
    "push esi",
    "mov esi, eax",
    "rep outsw",
    "pop esi",
    in("dx") port,
    in("eax") buf,
    in("ecx") count,
    );
}
