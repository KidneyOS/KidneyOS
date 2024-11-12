#![allow(dead_code)] // Suppress unused warnings

use crate::block::block_core::{Block, BlockSector, BlockType, BLOCK_SECTOR_SIZE};
use crate::block::block_error::BlockError;
use crate::block::partitions::partition_core::PartitionTable;
use crate::system::unwrap_system;
use kidneyos_shared::eprintln;

/// Register a partition on a block device.
///
/// # Safety
///
/// This function does not check for partition overlaps or other issues. It is up to the caller to
/// ensure that the partition is valid. For example, this function does not panic if we try to
/// register a partition that overlaps with an existing partition.
pub unsafe fn register_partition(
    p_start: BlockSector,
    p_size: BlockSector,
    p_type: BlockType,
    device: usize,
) -> Result<(), BlockError> {
    let block_manager = &unwrap_system().block_manager.read();
    let block_device = block_manager
        .by_id(device)
        .ok_or(BlockError::DeviceNotFound)?;

    if p_start + p_size > block_device.get_size() {
        return Err(BlockError::SectorOutOfBounds);
    }

    if p_type == BlockType::Swap {
        register_swap_partition(p_start, p_size, &block_device)
    } else {
        panic!("Registering partition of type {} not supported", p_type);
    }
}

fn register_swap_partition(
    p_start: BlockSector,
    p_size: BlockSector,
    device: &Block,
) -> Result<(), BlockError> {
    let mut buf: [u8; BLOCK_SECTOR_SIZE] = [0; BLOCK_SECTOR_SIZE];

    device.read(0, &mut buf)?;

    let mut pt = PartitionTable::new(&buf);
    let empty_entry = pt.entries.iter_mut().find(|e| e.is_empty());

    if empty_entry.is_none() {
        eprintln!("No empty partition entries found");
        return Err(BlockError::WriteError);
    }

    let entry = empty_entry.unwrap();
    // Bootable     0       +1      1
    entry.set_bootable(false);
    // Start        1       +3      4
    unsafe { entry.set_start(p_start) };
    // Type         4       +1      5       0x82: Linux Swap
    entry.set_partition_type(0x82);
    // End          5       +3      8       (set by set_size)
    // Offset       8       +4      12      (set by set_start)
    // Size         12      +4      16
    unsafe { entry.set_size(p_size) };

    // Write the partition table back to the disk
    buf = [0; BLOCK_SECTOR_SIZE]; // Clear the buffer, just in case
    pt.serialize(&mut buf);
    device.write(0, &buf)
}
