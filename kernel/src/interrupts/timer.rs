use super::mutex_irq::MutexIrq;
use crate::threading::scheduling::scheduler_yield_and_continue;
use core::time::Duration;

// PIT generates 3579545 / 3 Hz input signal which we wait to receive 0xffff (65535) of before sending a timer interrupt.
// This gives us an interval of 0xffff * 3 / 3579545 seconds between each timer interrupt
// https://wiki.osdev.org/Programmable_Interval_Timer
pub const TIMER_INTERRUPT_INTERVAL: Duration =
    Duration::from_micros((10u64).pow(6) * 0xffff * 3 / 3579545);

static SYS_CLOCK: MutexIrq<Duration> = MutexIrq::new(Duration::new(0, 0));

pub fn step_sys_clock() {
    let mut clock = SYS_CLOCK.lock();
    match clock.checked_add(TIMER_INTERRUPT_INTERVAL) {
        Some(update) => {
            *clock = update;
        }
        None => panic!("System clock overflowed!"),
    }
}

#[allow(unused)]
#[allow(clippy::while_immutable_condition)]
pub fn sleep(time: Duration) -> usize {
    let clock = SYS_CLOCK.lock();
    match clock.checked_add(time) {
        Some(end) => {
            while *clock < end {
                scheduler_yield_and_continue();
            }
            0
        }
        None => panic!("Wakeup time is too far into the future!"),
    }
}
