use core::arch::asm;

// QEMU default is 100 ticks per second
// This will need to be changed when compiling for a real system
pub const TICKS_PER_SECOND: u64 = 100;

pub const CLOCK_REALTIME: usize = 0;
pub const CLOCK_MONOTONIC: usize = 1;

#[derive(Debug)]
#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

fn tsc_to_timespec(tsc_high: u32, tsc_low: u32) -> Timespec {
    let tsc = ((tsc_high as u64) << 32) | (tsc_low as u64);

    let seconds = tsc / TICKS_PER_SECOND;
    let nanoseconds = (tsc % TICKS_PER_SECOND) * (1_000_000_000 / TICKS_PER_SECOND);

    Timespec {
        tv_sec: seconds as i64,
        tv_nsec: nanoseconds as i64,
    }
}

pub fn current_time() -> Timespec {
    let mut tsc_high: u32;
    let mut tsc_low: u32;

    unsafe {
        asm!(
            "rdtsc",
            lateout("eax") tsc_low,
            lateout("edx") tsc_high,
            options(nomem, nostack),
        );
    }

    tsc_to_timespec(tsc_high, tsc_low)
}

pub fn rtc_time() -> Timespec {
    let mut tsc_low: u32;
    let mut tsc_high: u32;

    unsafe {
        asm!(
            "
            in al, 0x70
            in al, 0x71
            ",
            lateout("eax") tsc_low,
            lateout("edx") tsc_high,
            options(nomem, nostack)
        );
    }

    tsc_to_timespec(tsc_high, tsc_low)
}
