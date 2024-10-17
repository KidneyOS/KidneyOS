// The code in the file is an interface to an ATA (IDE) controller. It attempts to comply to [ATA-3]
// Reference: https://wiki.osdev.org/ATA_PIO_Mode
// Reference: pintos/src/devices/ide.c

#![allow(dead_code)]

use crate::block::block_core::{BlockSector, BlockType, BLOCK_SECTOR_SIZE};
use crate::block::partitions::partition_core::partition_scan;
use crate::drivers::ata::ata_channel::AtaChannel;
use crate::drivers::ata::ata_device::AtaDevice;
use crate::interrupts::{intr_get_level, IntrLevel};
use crate::sync::mutex::sleep::SleepMutex;
use crate::system::unwrap_system_mut;
use alloc::boxed::Box;
use alloc::string::String;
use kidneyos_shared::println;
// Commands ----------------------------------------------------------------------------------------
// Reference: https://wiki.osdev.org/ATA_Command_Matrix

/// READ SECTOR (with retries)  PIO     8-bit
pub const ATA_READ_SECTOR_RETRY: u8 = 0x20;
/// WRITE SECTOR (with retries) PIO     8-bit
pub const ATA_WRITE_SECTOR_RETRY: u8 = 0x30;
/// IDENTIFY DEVICE             PIO     8-bit
pub const ATA_IDENTIFY_DEVICE: u8 = 0xEC;

// Constants ---------------------------------------------------------------------------------------

/// Number of ATA channels
const CHANNEL_CNT: usize = 2;
pub static CHANNELS: [SleepMutex<AtaChannel>; CHANNEL_CNT] = [
    SleepMutex::new(AtaChannel::new(0)),
    SleepMutex::new(AtaChannel::new(1)),
];

// -------------------------------------------------------------------------------------------------

/// Initialize the disk subsystem and detect disks.
///
/// # Safety
///
/// This function must be called with interrupts enabled.
pub fn ide_init() {
    assert_eq!(
        intr_get_level(),
        IntrLevel::IntrOn,
        "ide_init must be called with interrupts enabled"
    );

    println!("Initializing IDE subsystem");

    let mut present: [[bool; 2]; 2] = [[false; 2]; 2];

    for (i, c) in CHANNELS.iter().enumerate() {
        let channel = &mut c.lock();

        // Initialize the channel
        channel.set_names();
        unsafe { channel.reset(true) };

        // Initialize the devices
        if unsafe { channel.check_device_type(0, true) } {
            present[i][0] = true;
            present[i][1] = unsafe { channel.check_device_type(1, true) };
        } else {
            // println!("IDE: Channel {} device {} not ata", i, 0);
        }
    }

    for (i, c) in CHANNELS.iter().enumerate() {
        for j in 0..2 {
            if present[i][j] {
                unsafe { identify_ata_device(c, j as u8, true) };
            } else {
                // println!("IDE: Channel {} device {} not present", i, j);
            }
        }
    }

    println!("IDE subsystem initialized");
}

/// Sends an IDENTIFY DEVICE command to disk `dev_no` and reads the response. Registers the disk
/// with the block device layer.
///
/// # Safety
///
/// This function must be called with interrupts enabled
unsafe fn identify_ata_device(c: &SleepMutex<AtaChannel>, dev_no: u8, block: bool) {
    let _index: usize;
    let channel: &mut AtaChannel = &mut c.lock();
    let mut id: [u8; BLOCK_SECTOR_SIZE] = [0; BLOCK_SECTOR_SIZE];

    // Send the IDENTIFY DEVICE command, wait for an interrupt indicating the device's response
    // is ready, and read the data into our buffer.
    channel.select_device_wait(dev_no, block);
    channel.issue_pio_command(ATA_IDENTIFY_DEVICE);
    channel.sem_down();

    if !channel.wait_while_busy(block) {
        channel.set_is_ata(dev_no, false);
        // println!("channel {} device {} is not ata", c.channel_num, dev_no);
        return;
    }
    channel.read_sector(&mut id);

    // Calculate capacity.
    let capacity = usize::from_le_bytes(id[120..124].try_into().unwrap());
    let name = if dev_no == 0 {
        channel.get_d0_name()
    } else {
        channel.get_d1_name()
    };
    let name: String = name.iter().collect();
    println!(
        "channel: {} device: {} name: {} capacity: {}M",
        channel.get_channel_num(),
        dev_no,
        &name,
        capacity >> 11
    );

    let block_manager = &mut unwrap_system_mut().block_manager;

    let idx = block_manager.register_block(
        BlockType::Raw,
        &name,
        capacity as BlockSector,
        Box::new(AtaDevice(dev_no)),
    );

    partition_scan(block_manager.by_id(idx).unwrap());
}
