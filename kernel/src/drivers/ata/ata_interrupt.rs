use crate::drivers::ata::ata_core::CHANNELS;
use alloc::string::String;
use kidneyos_shared::println;
use kidneyos_shared::serial::inb;

pub fn on_ide_interrupt(vec_no: u8) {
    for (i, chan) in CHANNELS.iter().enumerate() {
        let c = &mut chan.lock();

        // Check if interrupt is from this channel
        if vec_no == c.get_irq() {
            // Check if channel is expecting an interrupt
            if c.is_expect_interrupt() {
                // Acknowledge the interrupt
                unsafe {
                    inb(c.reg_status());
                }
                // Wake up the waiting thread
                c.sem_up();
            } else {
                // Spurious interrupt
                println!(
                    "IDE: Spurious interrupt on channel {} ({})",
                    i,
                    String::from_iter(c.get_name())
                );
            }
        }
    }
}
