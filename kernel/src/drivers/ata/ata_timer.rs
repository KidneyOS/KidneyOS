// Wrapper for timer functions.

use crate::interrupts::timer::sleep;
use core::arch::asm;
use core::time::Duration;

/// Sleep for `t` milliseconds.
fn msleep_block(t: u64) {
    for _ in 0..9 {
        usleep_block(t);
    }
}

/// Sleep for `t` microseconds.
fn usleep_block(t: u64) {
    for _ in 0..9 * t {
        nsleep_block(t);
    }
}

/// Sleep for `t` nanoseconds.
fn nsleep_block(_t: u64) {
    unsafe {
        asm!("nop");
    }
}

/// Sleep for `t` milliseconds.
pub fn msleep(t: u64, block: bool) {
    if block {
        msleep_block(t);
    } else {
        sleep(Duration::from_millis(t));
    }
}

/// Sleep for `t` microseconds.
pub fn usleep(t: u64, block: bool) {
    if block {
        usleep_block(t);
    } else {
        sleep(Duration::from_micros(t));
    }
}

/// Sleep for `t` nanoseconds.
pub fn nsleep(t: u64, block: bool) {
    if block {
        nsleep_block(t);
    } else {
        sleep(Duration::from_nanos(t));
    }
}
