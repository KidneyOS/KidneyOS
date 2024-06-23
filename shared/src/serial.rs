use core::{arch::asm, fmt};

pub struct SerialWriter {
    initialized: bool,
}

const IO_BASE: u16 = 0x3f8;
const RBR: u16 = IO_BASE; // Receiver Buffer Reg (read-only)
const THR: u16 = IO_BASE; // Transmitter Holding Reg (write-only)
const IER: u16 = IO_BASE + 1; // Interrupt Enable Reg
const FCR: u16 = IO_BASE + 2; // FIFO Control Reg (write-only)
const LCR: u16 = IO_BASE + 3; // Line Control Register
const MCR: u16 = IO_BASE + 4; // MODEM Control Register
const LSR: u16 = IO_BASE + 5; // Line Status Register (read-only)

pub unsafe fn outb(port: u16, byte: u8) {
    asm!("out dx, al", in("dx") port, in("al") byte)
}

unsafe fn inb(port: u16) -> u8 {
    let res: u8;
    asm!("in al, dx", in("dx") port, out("al") res);
    res
}

impl SerialWriter {
    fn ensure_initialized(&mut self) {
        if self.initialized {
            return;
        }

        // SAFETY: Follows the correct proceedure for initializing serial ports.
        unsafe {
            // https://wiki.osdev.org/Serial_Ports#Initialization

            outb(IER, 0x00);
            outb(LCR, 0x80);
            outb(THR, 0x03);
            outb(IER, 0x00);
            outb(LCR, 0x03);
            outb(FCR, 0xC7);
            outb(MCR, 0x0B);

            outb(MCR, 0x1E); // Enable loopback.

            // Confirm that serial is working by writing a byte and reading it
            // back.
            const EXPECTED: u8 = 0xAE;
            outb(THR, EXPECTED);
            let actual = inb(RBR);
            assert!(
                actual == EXPECTED,
                "faulty serial, expected {EXPECTED:#X}, got {actual:#X}"
            );

            outb(MCR, 0x0F); // Disable loopback.

            self.initialized = true;
        }
    }
}

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // TODO: Once interrupts are enabled, do things the more efficient way.

        self.ensure_initialized();

        for b in s.bytes() {
            // SAFETY: Correctly waits before outputting byte to serial port.
            unsafe {
                while inb(LSR) & 0x20 == 0 {}
                outb(THR, b);
            }
        }

        Ok(())
    }
}

pub static mut SERIAL_WRITER: SerialWriter = SerialWriter { initialized: false };
