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

/// # Safety
///
/// Wrapper for the assembly function out.
pub unsafe fn outb(port: u16, byte: u8) {
    asm!("out dx, al", in("dx") port, in("al") byte)
}

/// # Safety
///
/// Wrapper for the assembly function in.
pub unsafe fn inb(port: u16) -> u8 {
    let res: u8;
    asm!("in al, dx", in("dx") port, out("al") res);
    res
}

/// Wrapper for assembly function insw - input from port to string.
///
/// Input word from I/O port specified in DX into memory location specified in ES:EDI.
///
/// # Safety
///
/// * The caller must ensure that the port is a valid port to read from.
/// * They also need to ensure the buffer is valid and has enough space to store the data.
pub unsafe fn insw(port: u16, buffer: *mut u8, count: usize) {
    asm!(
    // Save EDI to restore it after the insw instruction.
    "push edi",
    // Load the buffer address into EDI.
    "mov edi, eax",
    // Invoke `insw` instruction.
    "rep insw",
    // Restore EDI.
    "pop edi",
    in("dx") port,
    in("eax") buffer,
    in("ecx") count,
    options(nostack, preserves_flags)
    );
}

/// Wrapper for assembly function outsw - output string to port.
///
/// Output word from memory location specified in DS:ESI to I/O port specified in DX
///
/// # Safety
///
/// The caller must ensure that the port is a valid port to write to.
/// They also need to ensure the buffer is valid and has appropriate size to write to the port.
pub unsafe fn outsw(port: u16, buffer: *const u8, count: usize) {
    asm!(
    // Save ESI to restore it after the outsw instruction.
    "push esi",
    // Load the buffer address into ESI.
    "mov esi, eax",
    // Invoke `outsw` instruction.
    "rep outsw",
    // Restore ESI.
    "pop esi",
    in("dx") port,
    in("eax") buffer,
    in("ecx") count,
    options(nostack, preserves_flags)
    );
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
            assert_eq!(
                actual, EXPECTED,
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
