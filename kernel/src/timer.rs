use crate::{sync::irq::MutexIrq, threading::scheduling::scheduler_yield_and_continue};
use core::{arch::asm, time::Duration};
use kidneyos_shared::serial::{inb, outb};

pub const PIC1_OFFSET: u8 = 0x20;
pub const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xa0;
const PIC2_DATA: u16 = 0xa1;

const ICW1_ICW4: u8 = 0x01; /* Indicates that ICW4 will be present */
const ICW1_INIT: u8 = 0x10; /* Initialization - required! */
const ICW4_8086: u8 = 0x01; /* 8086/88 (MCS-80/85) mode */

const PIC_EOI: u8 = 0x20; /* End-of-interrupt command code */

// PIT generates 3579545 / 3 Hz input signal which we wait to receive 0xffff (65535) of before sending a timer interrupt.
// This gives us an interval of 0xffff * 3 / 3579545 seconds between each timer interrupt
// https://wiki.osdev.org/Programmable_Interval_Timer
pub const TIMER_INTERRUPT_INTERVAL: Duration =
    Duration::from_micros((10u64).pow(6) * 0xffff * 3 / 3579545);

static SYS_CLOCK: MutexIrq<Duration> = MutexIrq::new(Duration::new(0, 0));

pub unsafe fn pic_remap(offset1: u8, offset2: u8) {
    // Send command: Begin 3-byte initialization sequence.
    outb(PIC1_CMD, ICW1_INIT + ICW1_ICW4);
    io_wait();
    outb(PIC2_CMD, ICW1_INIT + ICW1_ICW4);
    io_wait();

    // Send data 1: Set interrupt offset.
    outb(PIC1_DATA, offset1);
    io_wait();
    outb(PIC2_DATA, offset2);
    io_wait();

    // Byte 2: Configure chaining between PIC1 and PIC2.
    outb(PIC1_DATA, 4);
    io_wait();
    outb(PIC2_DATA, 2);
    io_wait();

    // Send data 3: Set mode.
    outb(PIC1_DATA, ICW4_8086);
    io_wait();
    outb(PIC2_DATA, ICW4_8086);
    io_wait();
}

pub unsafe fn init_pit() {
    // program the PIT
    // channel 0 (bit 6-7), lo/hi-byte (bit 4-5), rate generator (bit 1-3)
    outb(0x43, 0b00110100);

    asm!(
        "
        mov ax, 0xffff // (reload value)
        out 0x40, al // set low byte of PIT reload value
        mov al, ah
        out 0x40, al // set high byte of PIT reload value
        ",
    );

    // unmask and activate all IRQs
    outb(PIC1_DATA, 0x0);
    outb(PIC2_DATA, 0x0);
}

#[allow(unused)]
pub unsafe fn irq_mask(mut irq: u8) {
    let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
    if irq >= 8 {
        irq -= 8
    };
    let mask = inb(port) | (1 << irq);

    outb(port, mask);
}

#[allow(unused)]
pub unsafe fn irq_unmask(mut irq: u8) {
    let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
    if irq >= 8 {
        irq -= 8
    };
    let mask = inb(port) & !(1 << irq);

    outb(port, mask);
}

pub unsafe fn send_eoi(irq: u8) {
    if irq >= 8 {
        outb(PIC2_CMD, PIC_EOI);
    }

    outb(PIC1_CMD, PIC_EOI);
}

unsafe fn io_wait() {
    // http://wiki.osdev.org/Inline_Assembly/Examples#IO_WAIT
    outb(0x80, 0);
}

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

pub unsafe fn set_enabled(enable: bool) {
    if enable {
        irq_mask(0x0);
    } else {
        irq_unmask(0x0);
    }
}
