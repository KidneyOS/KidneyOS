use crate::block::block_core::{BlockOp, BlockSector, BLOCK_SECTOR_SIZE};
use crate::block::block_error::BlockError;
use crate::drivers::ata::ata_channel::AtaChannel;
use crate::drivers::ata::ata_core::{ACCESS_MUTEX, CHANNELS};

#[derive(Copy, Clone, PartialEq)]
pub struct AtaDevice(pub u8);

impl AtaDevice {
    /// Get the channel number of the device
    pub fn get_channel(&self) -> u8 {
        // Second last bit
        (self.0 >> 1) & 0x1
    }

    /// Get the device number of the device
    ///
    /// * 0: master
    /// * 1: slave
    pub fn get_device_num(&self) -> u8 {
        // Last bit
        self.0 & 0x1
    }
}

impl BlockOp for AtaDevice {
    /// Reads `sector` from the disk into `buf`, which must have room for BLOCK_SECTOR_SIZE bytes.
    ///
    /// Internally synchronizes access to disks, so external per-disk locking is unneeded.
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled
    unsafe fn read(&mut self, sector: BlockSector, buf: &mut [u8]) -> Result<(), BlockError> {
        assert_eq!(buf.len(), BLOCK_SECTOR_SIZE); // Checked by block layer, should never fail

        let guard = ACCESS_MUTEX[self.get_channel() as usize].lock();
        let channel: &mut AtaChannel = &mut CHANNELS[self.get_channel() as usize];

        channel.select_sector(self.get_device_num(), sector, true);
        channel.issue_pio_command(crate::drivers::ata::ata_core::ATA_READ_SECTOR_RETRY);

        channel.sem_down();
        if !channel.wait_while_busy(true) {
            // println!("Read failed on sector {}.", sector);
            return Err(BlockError::ReadError);
        }
        channel.read_sector(buf);

        drop(guard);
        Ok(())
    }

    /// Write sector `sector` to the disk from `buf`, which must contain BLOCK_SECTOR_SIZE bytes.
    ///
    /// Returns after the disk has acknowledged receiving the data.
    ///
    /// Internally synchronizes access to disks, so external per-disk locking is unneeded.
    ///
    /// # Safety
    ///
    /// This function must be called with interrupts enabled
    unsafe fn write(&mut self, sector: BlockSector, buf: &[u8]) -> Result<(), BlockError> {
        assert_eq!(buf.len(), BLOCK_SECTOR_SIZE); // Checked by block layer, should never fail

        let guard = ACCESS_MUTEX[self.get_channel() as usize].lock();
        let channel: &mut AtaChannel = &mut CHANNELS[self.get_channel() as usize];

        channel.select_sector(self.get_device_num(), sector, true);
        channel.issue_pio_command(crate::drivers::ata::ata_core::ATA_WRITE_SECTOR_RETRY);

        if !channel.wait_while_busy(true) {
            // println!("Write failed on sector {}.", sec_no);
            return Err(BlockError::WriteError);
        }
        channel.write_sector(buf);
        channel.sem_down();

        drop(guard);
        Ok(())
    }
}
