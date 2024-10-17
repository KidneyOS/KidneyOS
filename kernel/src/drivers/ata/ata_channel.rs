#![allow(dead_code)] // Suppress unused warnings

use crate::block::block_core::{BlockSector, BLOCK_SECTOR_SIZE};
use crate::sync::semaphore::Semaphore;
use alloc::string::String;
use kidneyos_shared::println;
use kidneyos_shared::serial::{inb, insw, outb, outsw};

use crate::drivers::ata::ata_timer::{msleep, nsleep, usleep};

// Error Register bits -----------------------------------------------------------------------------
// Reference: https://wiki.osdev.org/ATA_PIO_Mode#Error_Register
/// 0   AMNF
///
/// Address mark not found
const ERR_AMNF: u8 = 0x01;
/// 1   TKZNF
///
/// Track 0 not found
const ERR_TKZNF: u8 = 0x02;
/// 2   ABRT
///
/// Aborted command
const ERR_ABRT: u8 = 0x04;
/// 3   MCR
///
/// Media change request
const ERR_MCR: u8 = 0x08;
/// 4   IDNF
///
/// ID not found
const ERR_IDNF: u8 = 0x10;
/// 5   MC
///
/// Media changed
const ERR_MC: u8 = 0x20;
/// 6   UNC
///
/// Uncorrectable data error
const ERR_UNC: u8 = 0x40;
/// 7   BBK
///
/// Bad block detected
const ERR_BBK: u8 = 0x80;

// Device Register bits ----------------------------------------------------------------------------
// Reference: https://wiki.osdev.org/ATA_PIO_Mode#Drive_/_Head_Register_(I/O_base_+_6)

/// 4   DRV     Device
///
/// Selects the drive number. 0 = master, 1 = slave
const DEV_DRV: u8 = 0x10;
/// 6   LBA     Linear Block Addressing
///
/// Uses CHS addressing if clear or LBA addressing if set.
const DEV_LBA: u8 = 0x40;
/// 5&7         Must Be Set
const DEV_MBS: u8 = 0xa0;

// Alternate Status Register Bits ------------------------------------------------------------------
// Reference: https://wiki.osdev.org/ATA_PIO_Mode#Status_Register_(I/O_base_+_7)

/// 0   ERR     Error
///
/// Indicates an error occurred. Send a new command to clear it (or nuke it with a Software Reset).
const STA_ERR: u8 = 0x01;
/// 1   IDX     Index
///
/// Index. Always set to zero.
const STA_IDX: u8 = 0x02;
/// 4   CORR    Corrected Data
///
/// Corrected data. Always set to zero.
const STA_CORR: u8 = 0x04;
/// 3   DRQ     Data Request
///
/// Set when the drive has PIO data to transfer, or is ready to accept PIO data.
const STA_DRQ: u8 = 0x08;
/// 4   SRV     Overlapped Mode Service Request
///
/// Overlapped Mode Service Request.
const STA_SRV: u8 = 0x10;
/// 5   DF      Drive Fault
///
/// Drive Fault Error (**does not set [ERR](STA_ERR)**).
const STA_DF: u8 = 0x20;
/// 6   RDY     Drive Ready
///
/// Bit is clear when drive is spun down, or after an error. Set otherwise.
const STA_RDY: u8 = 0x40;
/// 7   BSY     Busy
///
/// Indicates the drive is preparing to send/receive data (wait for it to clear). In case of 'hang'
/// (it never clears), do a software reset.
const STA_BSY: u8 = 0x80;

// Control Register bits ---------------------------------------------------------------------------
// Reference: https://wiki.osdev.org/ATA_PIO_Mode#Device_Control_Register_(Control_base_+_0)

/// 1   nIEN    Not Interrupt Enable
///
/// Set this to stop the current device from sending interrupts.
const CTL_NIEN: u8 = 0x02;
/// 2   SRST    Software Reset
///
/// Set, then clear (after 5us), this to do a "Software Reset" on all ATA drives on a bus, if one
/// is misbehaving.
const CTL_SRST: u8 = 0x04;
/// 7   HOB     High Order Byte
///
/// Set this to read back the High Order Byte of the last LBA48 value sent to an IO port.
const CTL_HOB: u8 = 0x80;

// Offsets -----------------------------------------------------------------------------------------

/// Control Base offset
/// Reference: https://lateblt.tripod.com/atapi.htm
/// 0x3F6 - 0x1F0 = 0x206
const CTL_OFFSET: u16 = 0x206;

// -------------------------------------------------------------------------------------------------

/// An ATA channel (aka controller)
///
/// Each channel can control up to two disks
pub struct AtaChannel {
    /// Name, e.g., "ide0"
    name: [char; 8],
    /// Base I/O port
    reg_base: u16,
    /// Interrupt in use
    irq: u8,

    /// True if an interrupt is expected, false if any interrupt would be spurious
    expecting_interrupt: bool,

    /// Up'd by interrupt handler
    completion_wait: Semaphore,

    /// The devices on this channel
    // Master
    d0_name: [char; 8],
    d0_is_ata: bool,
    // Slave
    d1_name: [char; 8],
    d1_is_ata: bool,

    channel_num: u8,
}

// ATA command block port addresses
// Reference: https://wiki.osdev.org/ATA_PIO_Mode#Registers
impl AtaChannel {
    /// R/W Data Register
    ///
    /// Read/Write PIO **data** bytes
    pub const fn reg_data(&self) -> u16 {
        self.reg_base
    }

    /// R   Error Register
    ///
    /// Used to retrieve any error generated by the last ATA command executed.
    pub const fn reg_error(&self) -> u16 {
        self.reg_base + 1
    }

    /// R/W Sector Count Register
    ///
    /// Number of sectors to read/write (0 is a special value).
    pub const fn reg_nsect(&self) -> u16 {
        self.reg_base + 2
    }

    /// R/W Sector Number Register (LBAlo)
    ///
    /// This is CHS / LBA28 / LBA48 specific.
    pub const fn reg_lbal(&self) -> u16 {
        self.reg_base + 3
    }

    /// R/W Cylinder Low Register (LBAmid)
    ///
    /// Partial Disk Sector address.
    pub const fn reg_lbam(&self) -> u16 {
        self.reg_base + 4
    }

    /// R/W Cylinder High Register (LBAhi)
    ///
    /// Partial Disk Sector address.
    pub const fn reg_lbah(&self) -> u16 {
        self.reg_base + 5
    }

    /// R   Device / Head Register
    ///
    /// Used to select a drive and/or head. Supports extra address/flag bits.
    pub const fn reg_device(&self) -> u16 {
        self.reg_base + 6
    }

    /// R   Status Register
    ///
    /// Used to read the current status.
    pub const fn reg_status(&self) -> u16 {
        self.reg_base + 7
    }

    /// W   Command Register
    ///
    /// Used to send ATA commands to the device.
    pub const fn reg_command(&self) -> u16 {
        self.reg_base + 7
    }
}

// ATA control block port addresses
// Reference: https://wiki.osdev.org/ATA_PIO_Mode#Registers
impl AtaChannel {
    /// R   Alternate Status Register
    ///
    /// A duplicate of the Status Register which does not affect interrupts.
    pub const fn reg_alt_status(&self) -> u16 {
        self.reg_base + CTL_OFFSET
    }

    /// W   Device Control Register
    ///
    /// Used to reset the bus or enable/disable interrupts.
    pub const fn reg_ctl(&self) -> u16 {
        self.reg_base + CTL_OFFSET
    }
}

impl AtaChannel {
    /// Resets the ATA channel and waits for any devices present on it to finish the reset
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled.
    pub unsafe fn reset(&mut self, block: bool) {
        let mut present: [bool; 2] = [false; 2];

        // The ATA reset sequence depends on which devices are present,
        // so we start by detecting device presence
        for dev_num in 0..2 {
            self.select_device(dev_num, block);

            // 0x55: 01010101
            // 0xaa: 10101010

            outb(self.reg_nsect(), 0x55);
            outb(self.reg_lbal(), 0xaa);

            outb(self.reg_nsect(), 0xaa);
            outb(self.reg_lbal(), 0x55);

            outb(self.reg_nsect(), 0x55);
            outb(self.reg_lbal(), 0xaa);

            present[dev_num as usize] =
                (inb(self.reg_nsect()) == 0x55) && inb(self.reg_lbal()) == 0xaa;
        }

        // Issue soft reset sequence, which selects device 0 as a side effect.
        // Also enable interrupts
        outb(self.reg_ctl(), 0);
        usleep(10, block);
        outb(self.reg_ctl(), CTL_SRST);
        usleep(10, block);
        outb(self.reg_ctl(), 0);

        msleep(150, block);

        // Wait for device 0 to clear BSY
        if present[0] {
            self.select_device(0, block);
            self.wait_while_busy(block);
        }
        // Wait for device 1 to clear BSY
        if present[1] {
            self.select_device(1, block);

            // Wait for 30 seconds for the device to spin up
            for _ in 0..3000 {
                if inb(self.reg_nsect()) == 1 && inb(self.reg_lbal()) == 1 {
                    break;
                }
                msleep(10, block);
            }
            self.wait_while_busy(block);
        }
    }

    /// Checks whether device `dev_num` is an ATA disk and set `dev_num`'s is_ata member
    /// appropriately.
    ///
    /// If `dev_num` is device 0 (master), returns true if it's possible that a slave (device 1)
    /// exists on this channel. If `dev_num` is 1 (slave), the return value is not meaningful.
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled.
    pub unsafe fn check_device_type(&mut self, dev_num: u8, block: bool) -> bool {
        self.select_device(dev_num, block);

        let error: u8 = inb(self.reg_error());
        let lbam: u8 = inb(self.reg_lbam());
        let lbah: u8 = inb(self.reg_lbah());
        let status: u8 = inb(self.reg_status());

        if (error != ERR_AMNF && (error != (ERR_AMNF | ERR_BBK) || dev_num == 1))
            // Device not ready
            || (status & STA_RDY) == 0
            // Device is busy
            || (status & STA_BSY) != 0
        {
            self.set_is_ata(dev_num, false);
            // error != (ERR_AMNF | ERR_BBK)
            false // simply ignore device
        } else {
            // PATA: 0x0  & 0x0
            // SATA: 0x3c & 0xc3
            let is_ata: bool = (lbam == 0 && lbah == 0) || (lbam == 0x3c && lbah == 0xc3);
            self.set_is_ata(dev_num, is_ata);
            true
        }
    }

    /// Selects device `dev_no`, waiting for it to become ready, and then writes SEC_NO to the
    /// disk's selection registers. (We use LBA mode).
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled.
    pub unsafe fn select_sector(&self, dev_no: u8, sector: BlockSector, block: bool) {
        self.select_device_wait(dev_no, block);

        // https://wiki.osdev.org/ATA_PIO_Mode#28_bit_PIO

        // 1. Send 0xE0 for the "master" or 0xF0 for the "slave", ORed with the highest 4 bits of
        // the LBA to port 0x1F6: outb(0x1F6, 0xE0 | (slavebit << 4) | ((LBA >> 24) & 0x0F))
        let device =
            DEV_MBS | DEV_LBA | if dev_no == 1 { DEV_DRV } else { 0 } | (sector >> 24) as u8;
        outb(self.reg_device(), device);

        // 2. Send a NULL byte to port 0x1F1, if you like (it is ignored and wastes lots of CPU
        // time): outb(0x1F1, 0x00)

        // 3. Send the sectorcount to port 0x1F2: outb(0x1F2, (unsigned char) count)
        outb(self.reg_nsect(), 1);

        // 4. Send the low 8 bits of the LBA to port 0x1F3: outb(0x1F3, (unsigned char) LBA))
        outb(self.reg_lbal(), sector as u8);

        // 5. Send the next 8 bits of the LBA to port 0x1F4: outb(0x1F4, (unsigned char)(LBA >> 8))
        outb(self.reg_lbam(), (sector >> 8) as u8);

        // 6. Send the next 8 bits of the LBA to port 0x1F5: outb(0x1F5, (unsigned char)(LBA >> 16))
        outb(self.reg_lbah(), (sector >> 16) as u8);
    }

    /// Writes `command` to the channel and prepares for receiving a completion interrupt.
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled.
    pub unsafe fn issue_pio_command(&mut self, command: u8) {
        self.expecting_interrupt = true;
        outb(self.reg_command(), command);
    }

    /// Reads a sector from the channel's data register in PIO mode into `buf`, which must have
    /// room for BLOCK_SECTOR_SIZE bytes.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `buf` is valid and has room for BLOCK_SECTOR_SIZE bytes.
    pub unsafe fn read_sector(&self, buf: &mut [u8]) {
        insw(self.reg_data(), buf.as_mut_ptr(), BLOCK_SECTOR_SIZE / 2);
    }

    /// Writes a sector to the channel's data register in PIO mode from `buf`, which must contain
    /// BLOCK_SECTOR_SIZE bytes.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `buf` is valid and contains BLOCK_SECTOR_SIZE bytes.
    pub unsafe fn write_sector(&mut self, buf: &[u8]) {
        outsw(self.reg_data(), buf.as_ptr(), BLOCK_SECTOR_SIZE / 2);
    }
}

// Low level ATA primitives
impl AtaChannel {
    /// Wait up to 10 seconds for the channel to become idle, that is, for the BSY and DRQ bits to
    /// clear in the status register.
    ///
    /// As a side effect, reading the status register clears any pending interrupt.
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled.
    pub unsafe fn wait_until_ready(&self, block: bool) {
        for _ in 0..1000 {
            if (inb(self.reg_status()) & (STA_BSY | STA_DRQ)) == 0 {
                return;
            }
            usleep(10, block);
        }

        println!("{} idle timeout", String::from_iter(&self.name));
    }

    /// Wait up to 30 seconds for the channel to clear BSY, and then return the status of the DRQ
    /// bit.
    ///
    /// The ATA standards say that a disk may take as long as that to complete its reset.
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled.
    pub unsafe fn wait_while_busy(&self, block: bool) -> bool {
        for i in 0..3000 {
            if i == 700 {
                println!("{} busy, waiting...", String::from_iter(&self.name));
            }

            if (inb(self.reg_alt_status()) & STA_BSY) == 0 {
                if i >= 700 {
                    kidneyos_shared::println!("{} ok", String::from_iter(&self.name));
                }
                return (inb(self.reg_alt_status()) & STA_DRQ) != 0;
            }
            usleep(10, block);
        }

        println!("{} wait_while_busy: failed", String::from_iter(&self.name));
        false
    }

    /// Program the channel so that `dev_num` is now the selected disk.
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled.
    pub unsafe fn select_device(&self, dev_num: u8, block: bool) {
        // Must be set + Device
        let dev: u8 = DEV_MBS | if dev_num == 1 { DEV_DRV } else { 0 };

        outb(self.reg_device(), dev);
        inb(self.reg_alt_status());

        nsleep(400, block);
    }

    /// Select disk `dev_num`, as [`AtaChannel::select_device`], but wait for the channel to become
    /// idle before and after.
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled.
    pub unsafe fn select_device_wait(&self, dev_num: u8, block: bool) {
        self.wait_until_ready(block);
        self.select_device(dev_num, block);
        self.wait_until_ready(block);
    }
}

impl AtaChannel {
    /* ATA command block port addresses */
    pub fn new(channel_num: u8) -> AtaChannel {
        let name: [char; 8] = ['\0'; 8];

        // https://wiki.osdev.org/ATA_PIO_Mode#Primary.2FSecondary_Bus
        let reg_base = match channel_num {
            // Primary ATA Bus: 0x1F0 - 0x1F7
            0 => 0x1F0,
            // Secondary ATA Bus: 0x170 - 0x177
            1 => 0x170,
            // Invalid
            _ => panic!(),
        };
        let irq = match channel_num {
            // Primary ATA Bus: IRQ 14
            // 0 => 0x20 + 14,
            0 => 14,
            // Secondary ATA Bus: IRQ 15
            // 1 => 0x20 + 15,
            1 => 15,
            // Invalid
            _ => panic!(),
        };

        let d0_name: [char; 8] = ['\0'; 8];
        let d1_name: [char; 8] = ['\0'; 8];

        AtaChannel {
            name,
            reg_base,
            irq,
            expecting_interrupt: false,
            completion_wait: Semaphore::new(0),
            d0_name,
            d0_is_ata: false,
            d1_name,
            d1_is_ata: false,
            channel_num,
        }
    }

    /// Sets the name, d0_name, and d1_name to the appropriate values based on the channel number.
    pub fn set_names(&mut self) {
        let name_char = char::from(b'0' + self.channel_num);
        let d0_char = char::from(b'a' + self.channel_num * 2);
        let d1_char = char::from(b'a' + 1 + self.channel_num * 2);

        self.name = ['i', 'd', 'e', name_char, '\0', '\0', '\0', '\0'];
        self.d0_name = ['h', 'd', d0_char, '\0', '\0', '\0', '\0', '\0'];
        self.d1_name = ['h', 'd', d1_char, '\0', '\0', '\0', '\0', '\0'];
    }

    /// Sets the is_ata member of the `dev_no` disk to `is_ata`.
    pub fn set_is_ata(&mut self, dev_no: u8, is_ata: bool) {
        if dev_no == 0 {
            self.d0_is_ata = is_ata;
        } else if dev_no == 1 {
            self.d1_is_ata = is_ata;
        } else {
            panic!(
                "{}.set_is_ata: invalid dev_no ({})",
                String::from_iter(&self.name),
                dev_no
            );
        }
    }

    /// Returns true if the `dev_no` disk is an ATA disk, false if it is not present or not an ATA
    /// disk.
    pub fn is_ata(&self, dev_no: u8) -> bool {
        if dev_no == 0 {
            self.d0_is_ata
        } else if dev_no == 1 {
            self.d1_is_ata
        } else {
            false
        }
    }

    pub fn get_channel_num(&self) -> u8 {
        self.channel_num
    }

    pub fn get_name(&self) -> [char; 8] {
        self.name
    }

    pub fn get_d0_name(&self) -> [char; 8] {
        self.d0_name
    }

    pub fn get_d1_name(&self) -> [char; 8] {
        self.d1_name
    }

    pub fn get_irq(&self) -> u8 {
        self.irq
    }

    pub fn is_expect_interrupt(&self) -> bool {
        self.expecting_interrupt
    }

    pub fn sem_down(&self) {
        self.completion_wait.acquire().forget();
    }

    pub fn sem_up(&self) {
        self.completion_wait.post();
    }

    pub fn sem_try_down(&self) -> bool {
        self.completion_wait.try_acquire().map(|x| x.forget()).is_some()
    }
}
