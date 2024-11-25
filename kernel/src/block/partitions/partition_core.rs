#![allow(dead_code)] // Suppress unused warnings, especially for the getters and setters

use crate::block::block_core::{Block, BlockOp, BlockSector, BlockType, BLOCK_SECTOR_SIZE};
use crate::block::block_error::BlockError;
use crate::block::partitions::partition_utils::lba_to_chs;
use crate::rush::rush_core::IS_SYSTEM_FULLY_INITIALIZED;
use crate::system::unwrap_system;
use alloc::boxed::Box;
use alloc::format;
use core::fmt;
use core::sync::atomic::Ordering::SeqCst;
use kidneyos_shared::{eprintln, println};
use crate::interrupts::{intr_disable, intr_enable, intr_get_level, IntrLevel};
use crate::interrupts::mutex_irq::hold_interrupts;

/// A partition table entry in the MBR.
///
/// Reference: https://wiki.osdev.org/MBR_(x86)#Partition_table_entry_format
pub(crate) struct PartitionTableEntry {
    /// 0x00    1   Drive attributes (bit 7 set = active or bootable)
    bootable: u8,

    /// 0x01    3   CHS Address of partition start
    start_cylinder: u8,
    start_head: u8,
    start_sector: u8,

    /// 0x04    1   Partition type
    partition_type: u8,

    /// 0x05    3   CHS address of last partition sector
    end_cylinder: u8,
    end_head: u8,
    end_sector: u8,

    /// 0x08    4   LBA of partition start
    offset: u32,
    /// 0x0C    4   Number of sectors in partition
    size: u32,
}

// Getters and setters
impl PartitionTableEntry {
    /// Get the bootable flag.
    pub(crate) fn get_bootable(&self) -> u8 {
        self.bootable
    }

    /// Set the bootable flag.
    pub(crate) fn set_bootable(&mut self, bootable: bool) {
        self.bootable = if bootable { 0x01 } else { 0x00 };
    }

    /// Get the cylinder of the start CHS address.
    pub(crate) fn get_start_cylinder(&self) -> u8 {
        self.start_cylinder
    }

    /// Set the cylinder of the start CHS address.
    ///
    /// Calling this function is discouraged. Use [`PartitionTableEntry::set_start`] instead.
    pub(crate) fn set_start_cylinder(&mut self, start_cylinder: u8) {
        self.start_cylinder = start_cylinder;
    }

    /// Get the head of the start CHS address.
    pub(crate) fn get_start_head(&self) -> u8 {
        self.start_head
    }

    /// Set the head of the start CHS address.
    ///
    /// Calling this function is discouraged. Use [`PartitionTableEntry::set_start`] instead.
    pub(crate) fn set_start_head(&mut self, start_head: u8) {
        self.start_head = start_head;
    }

    /// Get the sector of the start CHS address.
    pub(crate) fn get_start_sector(&self) -> u8 {
        self.start_sector
    }

    /// Set the sector of the start CHS address.
    ///
    /// Calling this function is discouraged. Use [`PartitionTableEntry::set_start`] instead.
    pub(crate) fn set_start_sector(&mut self, start_sector: u8) {
        self.start_sector = start_sector;
    }

    /// Set the start CHS address and update the offset.
    ///
    /// # Safety
    ///
    /// After calling this function, either `size` or `end` must be updated to avoid inconsistencies.
    ///
    /// # Important
    ///
    /// Despite the name, this function also updates the `offset` to avoid inconsistencies. This
    /// function is mutually exclusive with [`PartitionTableEntry::set_offset`], and it suffices to
    /// call one of them.
    pub(crate) unsafe fn set_start(&mut self, start: BlockSector) {
        let (cylinder, head, sector) = lba_to_chs(start);
        self.start_cylinder = cylinder;
        self.start_head = head;
        self.start_sector = sector;

        // Also update the offset
        self.offset = start;
    }

    /// Get the partition type.
    ///
    /// The partition type is a number that represents the type of the partition.
    /// To get the name of the partition type, see the [`partition_type_name`] function.
    pub(crate) fn get_partition_type(&self) -> u8 {
        self.partition_type
    }

    /// Set the partition type.
    pub(crate) fn set_partition_type(&mut self, partition_type: u8) {
        self.partition_type = partition_type;
    }

    /// Get the cylinder of the end CHS address.
    pub(crate) fn get_end_cylinder(&self) -> u8 {
        self.end_cylinder
    }

    /// Set the cylinder of the end CHS address.
    ///
    /// Calling this function is discouraged. Use [`PartitionTableEntry::set_end`] instead.
    pub(crate) fn set_end_cylinder(&mut self, end_cylinder: u8) {
        self.end_cylinder = end_cylinder;
    }

    /// Get the head of the end CHS address.
    pub(crate) fn get_end_head(&self) -> u8 {
        self.end_head
    }

    /// Set the head of the end CHS address.
    ///
    /// Calling this function is discouraged. Use [`PartitionTableEntry::set_end`] instead.
    pub(crate) fn set_end_head(&mut self, end_head: u8) {
        self.end_head = end_head;
    }

    /// Get the sector of the end CHS address.
    pub(crate) fn get_end_sector(&self) -> u8 {
        self.end_sector
    }

    /// Set the sector of the end CHS address.
    ///
    /// Calling this function is discouraged. Use [`PartitionTableEntry::set_end`] instead.
    pub(crate) fn set_end_sector(&mut self, end_sector: u8) {
        self.end_sector = end_sector;
    }

    /// Set the end CHS address and update the size.
    ///
    /// # Safety
    ///
    /// This function must be called after setting the `offset` to avoid inconsistencies.
    ///
    /// # Important
    ///
    /// Despite the name, this function also updates the `size` to avoid inconsistencies. This
    /// function is mutually exclusive with [`PartitionTableEntry::set_size`], and it suffices to
    /// call one of them.
    pub(crate) unsafe fn set_end(&mut self, end: BlockSector) {
        let (cylinder, head, sector) = lba_to_chs(end);
        self.end_cylinder = cylinder;
        self.end_head = head;
        self.end_sector = sector;

        // Also update the size
        self.size = end - self.offset;
    }

    /// Get the offset.
    pub(crate) fn get_offset(&self) -> u32 {
        self.offset
    }

    /// Set the offset.
    ///
    /// # Safety
    ///
    /// After calling this function, either `size` or `end` must be updated to avoid inconsistencies.
    ///
    /// # Important
    ///
    /// Despite the name, this function also updates the `start` to avoid inconsistencies. This
    /// function is mutually exclusive with [`PartitionTableEntry::set_start`], and it suffices to
    /// call one of them.
    pub(crate) unsafe fn set_offset(&mut self, offset: u32) {
        self.offset = offset;

        // Also update the start
        self.set_start(offset);
    }

    /// Get the size.
    pub(crate) fn get_size(&self) -> u32 {
        self.size
    }

    /// Set the size.
    ///
    /// # Safety
    ///
    /// This function must be called after setting the `offset` to avoid inconsistencies.
    ///
    /// # Important
    ///
    /// Despite the name, this function also updates the `end` to avoid inconsistencies. This
    /// function is mutually exclusive with [`PartitionTableEntry::set_end`], and it suffices to
    /// call one of them.
    pub(crate) unsafe fn set_size(&mut self, size: u32) {
        self.size = size;

        // Also update the end
        self.set_end(self.offset + size);
    }
}

impl fmt::Display for PartitionTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "bootable: {}, start: {}:{}:{}, type: {}, end: {}:{}:{}, offset: {}, size: {}",
            self.bootable,
            self.start_cylinder,
            self.start_head,
            self.start_sector,
            partition_type_name(self.partition_type),
            self.end_cylinder,
            self.end_head,
            self.end_sector,
            self.offset,
            self.size
        )
    }
}

impl PartitionTableEntry {
    pub(crate) fn new(buf: &[u8]) -> PartitionTableEntry {
        let bootable = buf[0];
        let start_cylinder = buf[1];
        let start_head = buf[2];
        let start_sector = buf[3];
        let partition_type = buf[4];
        let end_cylinder = buf[5];
        let end_head = buf[6];
        let end_sector = buf[7];
        let offset = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let size = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);

        PartitionTableEntry {
            bootable,
            start_cylinder,
            start_head,
            start_sector,
            partition_type,
            end_cylinder,
            end_head,
            end_sector,
            offset,
            size,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.size == 0 || self.partition_type == 0
    }

    pub(crate) fn serialize(&self, buf: &mut [u8]) {
        // Bootable     0       +1      1
        buf[0] = self.bootable;

        // Start        1       +3      4
        // Cylinder     1       +1      2
        buf[1] = self.start_cylinder;
        // Head         2       +1      3
        buf[2] = self.start_head;
        // Sector       3       +1      4
        buf[3] = self.start_sector;

        // Type         4       +1      5
        buf[4] = self.partition_type;

        // End          5       +3      8
        // Cylinder     5       +1      6
        buf[5] = self.end_cylinder;
        // Head         6       +1      7
        buf[6] = self.end_head;
        // Sector       7       +1      8
        buf[7] = self.end_sector;

        // Offset       8       +4      12
        buf[8..12].copy_from_slice(&self.offset.to_le_bytes());

        // Size         12      +4      16
        buf[12..16].copy_from_slice(&self.size.to_le_bytes());
    }
}

/// An MBR partition table.
///
/// Reference: https://wiki.osdev.org/MBR_(x86)#MBR_format
pub(crate) struct PartitionTable {
    /// 0x000   440     MBR Bootstrap (flat binary executable code)
    ///
    /// This can be extended to 446 bytes if you omit the next 2 optional fields: Disk ID and
    /// reserved.
    pub bootstrap: [u8; 440],
    /// 0x1B8   4       Optional "Unique Disk ID / Signature"
    ///
    /// The 4 byte "Unique Disk ID" is used by recent Linux and Windows systems to identify the
    /// drive. "Unique" in this case means that the IDs of all the drives attached to a particular system are distinct.
    pub id: u32,
    /// 0x1BC   2       Optional, reserved 0x0000
    ///
    /// The 2 byte reserved is usually 0x0000. 0x5A5A means read-only according to
    /// https://neosmart.net/wiki/mbr-boot-process/
    pub reserved: u16,

    /// 0x1BE   16      First partition table entry
    /// 0x1CE   16      Second partition table entry
    /// 0x1DE   16      Third partition table entry
    /// 0x1EE   16      Fourth partition table entry
    pub entries: [PartitionTableEntry; 4],
    /// 0x1FE   2       (0x55, 0xAA) "Valid bootsector" signature bytes
    pub signature: u16,
}

impl PartitionTable {
    pub(crate) fn new(buf: &[u8]) -> PartitionTable {
        let mut bootstrap = [0; 440];
        bootstrap.copy_from_slice(&buf[0..440]);
        let id = u32::from_le_bytes([buf[440], buf[441], buf[442], buf[443]]);
        let reserved = u16::from_le_bytes([buf[444], buf[445]]);
        let entries = [
            PartitionTableEntry::new(&buf[446..462]),
            PartitionTableEntry::new(&buf[462..478]),
            PartitionTableEntry::new(&buf[478..494]),
            PartitionTableEntry::new(&buf[494..510]),
        ];
        let signature = u16::from_le_bytes([buf[510], buf[511]]);

        PartitionTable {
            bootstrap,
            id,
            reserved,
            entries,
            signature,
        }
    }

    pub(crate) fn serialize(&self, buf: &mut [u8]) {
        // Bootstrap    0       +440    440
        buf[0..440].copy_from_slice(&self.bootstrap);

        // Id           440     +4      444
        buf[440..444].copy_from_slice(&self.id.to_le_bytes());

        // Reserved     444     +2      446
        buf[444..446].copy_from_slice(&self.reserved.to_le_bytes());

        // Entries      446     +64     510
        let mut entry_buf: [u8; 16] = [0; 16];
        // Entry 1      446     +16     462
        self.entries[0].serialize(&mut entry_buf);
        buf[446..462].copy_from_slice(&entry_buf);
        // Entry 2      462     +16     478
        self.entries[1].serialize(&mut entry_buf);
        buf[462..478].copy_from_slice(&entry_buf);
        // Entry 3      478     +16     494
        self.entries[2].serialize(&mut entry_buf);
        buf[478..494].copy_from_slice(&entry_buf);
        // Entry 4      494     +16     510
        self.entries[3].serialize(&mut entry_buf);
        buf[494..510].copy_from_slice(&entry_buf);

        // Signature    510     +2      512
        buf[510..512].copy_from_slice(&self.signature.to_le_bytes());
    }
}

/// A partition.
pub struct Partition {
    block_idx: usize,
    start: BlockSector,
}

impl BlockOp for Partition {
    unsafe fn read(&mut self, sector: BlockSector, buf: &mut [u8]) -> Result<(), BlockError> {
        unwrap_system()
            .block_manager
            .read()
            .by_id(self.block_idx)
            .unwrap()
            .read(sector + self.start, buf)
    }

    unsafe fn write(&mut self, sector: BlockSector, buf: &[u8]) -> Result<(), BlockError> {
        unwrap_system()
            .block_manager
            .read()
            .by_id(self.block_idx)
            .unwrap()
            .write(sector + self.start, buf)
    }
}

pub fn partition_type_name(ty: u8) -> &'static str {
    match ty {
        0x00 => "Empty",
        0x01 => "FAT12",
        0x02 => "XENIX root",
        0x03 => "XENIX usr",
        0x04 => "FAT16 <32M",
        0x05 => "Extended",
        0x06 => "FAT16",
        0x07 => "HPFS/NTFS",
        0x08 => "AIX",
        0x09 => "AIX bootable",
        0x0a => "OS/2 Boot Manager",
        0x0b => "W95 FAT32",
        0x0c => "W95 FAT32 (LBA)",
        0x0e => "W95 FAT16 (LBA)",
        0x0f => "W95 Ext'd (LBA)",
        0x10 => "OPUS",
        0x11 => "Hidden FAT12",
        0x12 => "Compaq diagnostics",
        0x14 => "Hidden FAT16 <32M",
        0x16 => "Hidden FAT16",
        0x17 => "Hidden HPFS/NTFS",
        0x18 => "AST SmartSleep",
        0x1b => "Hidden W95 FAT32",
        0x1c => "Hidden W95 FAT32 (LBA)",
        0x1e => "Hidden W95 FAT16 (LBA)",
        0x20 => "Pintos OS kernel",
        0x21 => "Pintos file system",
        0x22 => "Pintos scratch",
        0x23 => "Pintos swap",
        0x24 => "NEC DOS",
        0x39 => "Plan 9",
        0x3c => "PartitionMagic recovery",
        0x40 => "Venix 80286",
        0x41 => "PPC PReP Boot",
        0x42 => "SFS",
        0x4d => "QNX4.x",
        0x4e => "QNX4.x 2nd part",
        0x4f => "QNX4.x 3rd part",
        0x50 => "OnTrack DM",
        0x51 => "OnTrack DM6 Aux1",
        0x52 => "CP/M",
        0x53 => "OnTrack DM6 Aux3",
        0x54 => "OnTrackDM6",
        0x55 => "EZ-Drive",
        0x56 => "Golden Bow",
        0x5c => "Priam Edisk",
        0x61 => "SpeedStor",
        0x63 => "GNU HURD or SysV",
        0x64 => "Novell Netware 286",
        0x65 => "Novell Netware 386",
        0x70 => "DiskSecure Multi-Boot",
        0x75 => "PC/IX",
        0x80 => "Old Minix",
        0x81 => "Minix / old Linux",
        0x82 => "Linux swap / Solaris",
        0x83 => "Linux",
        0x84 => "OS/2 hidden C: drive",
        0x85 => "Linux extended",
        0x86 => "NTFS volume set",
        0x87 => "NTFS volume set",
        0x88 => "Linux plaintext",
        0x8e => "Linux LVM",
        0x93 => "Amoeba",
        0x94 => "Amoeba BBT",
        0x9f => "BSD/OS",
        0xa0 => "IBM Thinkpad hibernation",
        0xa5 => "FreeBSD",
        0xa6 => "OpenBSD",
        0xa7 => "NeXTSTEP",
        0xa8 => "Darwin UFS",
        0xa9 => "NetBSD",
        0xab => "Darwin boot",
        0xb7 => "BSDI fs",
        0xb8 => "BSDI swap",
        0xbb => "Boot Wizard hidden",
        0xbe => "Solaris boot",
        0xbf => "Solaris",
        0xc1 => "DRDOS/sec (FAT-12)",
        0xc4 => "DRDOS/sec (FAT-16 < 32M)",
        0xc6 => "DRDOS/sec (FAT-16)",
        0xc7 => "Syrinx",
        0xda => "Non-FS data",
        0xdb => "CP/M / CTOS / ...",
        0xde => "Dell Utility",
        0xdf => "BootIt",
        0xe1 => "DOS access",
        0xe3 => "DOS R/O",
        0xe4 => "SpeedStor",
        0xeb => "BeOS fs",
        0xee => "EFI GPT",
        0xef => "EFI (FAT-12/16/32)",
        0xf0 => "Linux/PA-RISC boot",
        0xf1 => "SpeedStor",
        0xf4 => "SpeedStor",
        0xf2 => "DOS secondary",
        0xfd => "Linux raid autodetect",
        0xfe => "LANstep",
        0xff => "BBT",
        _ => "Unknown",
    }
}

pub fn partition_scan(block: &Block) {
    let mut part_nr = 0;
    read_partition_table(block, 0, 0, &mut part_nr);
    if part_nr == 0 {
        eprintln!("{}: Device contains no partitions", block.get_name());
    }

    IS_SYSTEM_FULLY_INITIALIZED.store(true, SeqCst);
}

fn read_partition_table(
    block: &Block,
    sector: BlockSector,
    primary_extended_sector: BlockSector,
    part_nr: &mut i32,
) {
    // Check sector validity
    if sector >= block.get_size() {
        eprintln!(
            "{}: Partition table at sector {} past end of device ({} sectors)",
            block.get_name(),
            sector,
            block.get_size()
        );
        return;
    }

    // Read sector
    let mut buf: [u8; BLOCK_SECTOR_SIZE] = [0; BLOCK_SECTOR_SIZE];

    // TODO: remove intr related coed when ata_core.rs:50 is fixed
    let intr_level = intr_get_level();
    intr_enable();
    let ret = block.read(sector, &mut buf);
    if intr_level == IntrLevel::IntrOff {
        intr_disable();
    }

    if ret.is_err() {
        eprintln!("{}: Error reading partition table", block.get_name());
        return;
    }

    let pt = PartitionTable::new(&buf);

    // Check signature
    if pt.signature != 0xAA55 {
        if primary_extended_sector == 0 {
            eprintln!("{}: Invalid partition table signature", block.get_name());
        } else {
            eprintln!(
                "{}: Invalid extended partition table in sector",
                block.get_name()
            );
        }
        return;
    }

    // Parse partitions
    for entry in pt.entries.iter() {
        if entry.size == 0 || entry.partition_type == 0 {
            continue;
        } else if entry.partition_type == 0x05
            || entry.partition_type == 0x0F
            || entry.partition_type == 0x85
            || entry.partition_type == 0xc5
        {
            eprintln!(
                "{}: Extended partition in sector {}",
                block.get_name(),
                sector
            );

            if sector == 0 {
                read_partition_table(block, entry.offset, entry.offset, part_nr);
            } else {
                read_partition_table(
                    block,
                    entry.offset + primary_extended_sector,
                    primary_extended_sector,
                    part_nr,
                );
            }
        } else {
            *part_nr += 1;

            found_partition(
                block,
                entry.partition_type,
                entry.offset + sector,
                entry.size,
                part_nr,
            );
        }
    }
}

fn found_partition(
    block: &Block,
    partition_type: u8,
    start: BlockSector,
    size: u32,
    part_nr: &mut i32,
) {
    if start >= block.get_size() {
        eprintln!(
            "{}: Partition {} starts at sector {} past end of device ({} sectors)",
            block.get_name(),
            part_nr,
            start,
            block.get_size()
        );
    } else if start.overflowing_add(size).1 || start + size > block.get_size() {
        eprintln!(
            "{}: Partition {} ends at sector {} past end of device ({} sectors)",
            block.get_name(),
            part_nr,
            start + size,
            block.get_size()
        );
    } else {
        let b_type: BlockType = match partition_type {
            0x20 => BlockType::Kernel,
            0x21 => BlockType::FileSystem,
            0x22 => BlockType::Scratch,
            0x23 => BlockType::Swap,
            _ => BlockType::Raw,
        };

        let name = format!("{}-{}", block.get_name(), part_nr);
        println!(
            "{}: Found partition {} ({}), {} to {}, {} sectors",
            block.get_name(),
            part_nr,
            partition_type_name(partition_type),
            start,
            start + size,
            size
        );

        let p = Partition {
            block_idx: block.get_index(),
            start,
        };
        unwrap_system().block_manager.write().register_block(
            b_type,
            name.as_ref(),
            size,
            Box::new(p),
        );
    }
}
