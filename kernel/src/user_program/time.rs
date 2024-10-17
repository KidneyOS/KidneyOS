use core::arch::asm;

// QEMU default is 100 ticks per second
pub const TICKS_PER_SECOND: u64 = 100;

#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

pub fn current_time() -> Timespec {
    let mut tsc_high: i32;
    let mut tsc_low: i32;

    unsafe {
        asm!(
            "rdtsc",
            out("eax") tsc_low,
            out("edx") tsc_high,
            options(nomem, nostack),
        );
    }

    // Combine EAX and EDX into a 64-bit value
    let tsc = ((tsc_high as u64) << 32) | (tsc_low as u64);

    // Convert TSC to seconds and nanoseconds
    let seconds = tsc / TICKS_PER_SECOND;
    let nanoseconds = (tsc % TICKS_PER_SECOND) * (1_000_000_000 / TICKS_PER_SECOND);

    Timespec {
        tv_sec: seconds as i64,
        tv_nsec: nanoseconds as i64,
    }
}
