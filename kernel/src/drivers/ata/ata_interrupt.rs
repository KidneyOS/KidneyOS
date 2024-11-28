use crate::drivers::ata::ata_core::CHANNELS;
use alloc::string::String;
use kidneyos_shared::serial::inb;
use kidneyos_shared::{eprintln, println};

pub fn on_ide_interrupt(vec_no: u8) {
    for (i, c) in CHANNELS.iter().enumerate() {
        let channel = &mut c.lock();

        // Check if interrupt is from this channel
        if vec_no == channel.get_irq() {
            // Check if channel is expecting an interrupt
            if channel.is_expect_interrupt() {
                // Acknowledge the interrupt
                unsafe {
                    inb(channel.reg_status());
                }
                // Wake up the waiting thread
                channel.sem_up();
            } else {
                // Spurious interrupt
                eprintln!(
                    "IDE: Spurious interrupt on channel {} ({})",
                    i,
                    String::from_iter(channel.get_name())
                );
            }
        }
    }
}
