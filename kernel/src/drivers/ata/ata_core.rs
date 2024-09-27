// The code in the file is an interface to an ATA (IDE) controller. It attempts to comply to [ATA-3]
// Reference: https://wiki.osdev.org/ATA_PIO_Mode
// Reference: pintos/src/devices/ide.c

#![allow(dead_code)]

use crate::block::block_core::{BlockManager, BlockSector, BlockType, BLOCK_SECTOR_SIZE};
use crate::drivers::ata::ata_channel::AtaChannel;
use crate::drivers::ata::ata_device::AtaDevice;
use crate::sync::mutex::Mutex;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use kidneyos_shared::println;
use lazy_static::lazy_static;

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
lazy_static! {
    /// A list of ATA channels
    pub static ref CHANNELS: Vec<Mutex<AtaChannel>> = {
        (0..CHANNEL_CNT)
            .map(|i| Mutex::new(AtaChannel::new(i as u8)))
            .collect()
    };
}

// -------------------------------------------------------------------------------------------------

/// Initialize the disk subsystem and detect disks.
///
/// # Safety
///
/// This function must be called with interrupts enabled.
pub unsafe fn ide_init(mut all_blocks: BlockManager, block: bool) -> BlockManager {
    let mut present: [[bool; 2]; 2] = [[false; 2]; 2];

    for (i, c) in CHANNELS.iter().enumerate() {
        let channel = &mut c.lock();

        // Initialize the channel
        channel.set_names();
        channel.reset(true);

        // Initialize the devices
        if channel.check_device_type(0, block) {
            present[i][0] = true;
            present[i][1] = channel.check_device_type(1, block);
        } else {
            println!("IDE: Channel {} device {} not ata", i, 0);
        }
    }

    for (i, c) in CHANNELS.iter().enumerate() {
        for j in 0..2 {
            if present[i][j] {
                all_blocks = identify_ata_device(c, j as u8, all_blocks, block);
            } else {
                println!("IDE: Channel {} device {} not present", i, j);
            }
        }
    }

    all_blocks
}

/// Sends an IDENTIFY DEVICE command to disk `dev_no` and reads the response. Registers the disk
/// with the block device layer.
///
/// # Safety
///
/// This function must be called with interrupts enabled
unsafe fn identify_ata_device(
    channel: &'static Mutex<AtaChannel>,
    dev_no: u8,
    mut all_blocks: BlockManager,
    block: bool,
) -> BlockManager {
    let _index: usize;
    let c: &mut AtaChannel = &mut channel.lock();
    let mut id: [u8; BLOCK_SECTOR_SIZE] = [0; BLOCK_SECTOR_SIZE];

    // Send the IDENTIFY DEVICE command, wait for an interrupt indicating the device's response
    // is ready, and read the data into our buffer.
    c.select_device_wait(dev_no, block);
    c.issue_pio_command(ATA_IDENTIFY_DEVICE);
    c.sem_down();

    if !c.wait_while_busy(block) {
        c.set_is_ata(dev_no, false);
        // println!("channel {} device {} is not ata", c.channel_num, dev_no);
        return all_blocks;
    }
    c.read_sector(&mut id);

    // Calculate capacity.
    let capacity = usize::from_le_bytes(id[120..124].try_into().unwrap());
    let name = if dev_no == 0 {
        c.get_d0_name()
    } else {
        c.get_d1_name()
    };
    let name: String = name.iter().collect();
    println!(
        "channel: {} device: {} name: {} capacity: {}M",
        c.get_channel_num(),
        dev_no,
        &name,
        capacity >> 11
    );

    // TODO: Register block device
    all_blocks.register_block(
        BlockType::Raw,
        &name,
        capacity as BlockSector,
        Box::new(AtaDevice(dev_no)),
    );

    // TODO: scan partitions and recognize block types
    all_blocks
}