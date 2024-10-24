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

// Convert the RTC time to a Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
fn rtc_to_unix_timestamp(
    year: i32,
    month: u8,
    day: u8,
    hours: u8,
    minutes: u8,
    seconds: u8,
) -> i64 {
    const DAYS_IN_MONTH: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut days = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    days += DAYS_IN_MONTH.iter().take((month - 1) as usize).sum::<i32>();

    if month > 2 && is_leap_year(year) {
        days += 1;
    }

    days += day as i32 - 1;

    days as i64 * 24 * 3600 + hours as i64 * 3600 + minutes as i64 * 60 + seconds as i64
}

// Check if a year is a leap year
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn get_tsc() -> Timespec {
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

    let tsc = ((tsc_high as u64) << 32) | (tsc_low as u64);

    let seconds = tsc / TICKS_PER_SECOND;
    let nanoseconds = (tsc % TICKS_PER_SECOND) * (1_000_000_000 / TICKS_PER_SECOND);

    Timespec {
        tv_sec: seconds as i64,
        tv_nsec: nanoseconds as i64,
    }
}

pub fn get_rtc() -> Timespec {
    let mut seconds: u8;
    let mut minutes: u8;
    let mut hours: u8;
    let mut day: u8;
    let mut month: u8;
    let mut year: u8;

    unsafe {
        // Wait for the RTC to not be updating
        asm!(
            "2:",
            "mov al, 0x0A",  // Load RTC register A
            "out 0x70, al",  // Output to address 0x70
            "in al, 0x71",   // Read from RTC register A
            "test al, 0x80", // Check if update is in progress (bit 7)
            "jne 2b",        // Loop if update is in progress
        );

        // seconds
        asm!(
            "mov al, 0x00",
            "out 0x70, al",
            "in al, 0x71",
            out("al") seconds
        );

        // minutes
        asm!(
            "mov al, 0x02",
            "out 0x70, al",
            "in al, 0x71",
            out("al") minutes
        );

        // hours
        asm!(
            "mov al, 0x04",
            "out 0x70, al",
            "in al, 0x71",
            out("al") hours
        );

        // day of month
        asm!(
            "mov al, 0x07",
            "out 0x70, al",
            "in al, 0x71",
            out("al") day
        );

        // month
        asm!(
            "mov al, 0x08",
            "out 0x70, al",
            "in al, 0x71",
            out("al") month
        );

        // year
        asm!(
            "mov al, 0x09",
            "out 0x70, al",
            "in al, 0x71",
            out("al") year
        );
    }

    let full_year = 2000 + year as i32;

    let unix_time = rtc_to_unix_timestamp(full_year, month, day, hours, minutes, seconds);

    Timespec {
        tv_sec: unix_time,
        tv_nsec: 0,
    }
}
