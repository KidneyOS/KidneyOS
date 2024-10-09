use crate::block::block_core::{
    Block, BlockOp, BlockSector, BlockType, BLOCK_MANAGER, BLOCK_SECTOR_SIZE,
};
use crate::block::block_error::BlockError;
use crate::interrupts::{intr_disable, intr_enable, intr_get_level, IntrLevel};
use alloc::boxed::Box;
use alloc::format;
use kidneyos_shared::{eprintln, println};

/// A partition table entry in the MBR.
///
/// Reference: https://wiki.osdev.org/MBR_(x86)#Partition_table_entry_format
struct PartitionTableEntry {
    /// 0x00    1   Drive attributes (bit 7 set = active or bootable)
    _bootable: u8,

    /// 0x01    3   CHS Address of partition start
    _start_cylinder: u8,
    _start_head: u8,
    _start_sector: u8,

    /// 0x04    1   Partition type
    partition_type: u8,

    /// 0x05    3   CHS address of last partition sector
    _end_cylinder: u8,
    _end_head: u8,
    _end_sector: u8,

    /// 0x08    4   LBA of partition start
    offset: u32,
    /// 0x0C    4   Number of sectors in partition
    size: u32,
}

impl PartitionTableEntry {
    fn new(buf: &[u8]) -> PartitionTableEntry {
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
            _bootable: bootable,
            _start_cylinder: start_cylinder,
            _start_head: start_head,
            _start_sector: start_sector,
            partition_type,
            _end_cylinder: end_cylinder,
            _end_head: end_head,
            _end_sector: end_sector,
            offset,
            size,
        }
    }
}

/// An MBR partition table.
///
/// Reference: https://wiki.osdev.org/MBR_(x86)#MBR_format
struct PartitionTable {
    /// 0x000   440     MBR Bootstrap (flat binary executable code)
    ///
    /// This can be extended to 446 bytes if you omit the next 2 optional fields: Disk ID and
    /// reserved.
    pub _bootstrap: [u8; 440],
    /// 0x1B8   4       Optional "Unique Disk ID / Signature"
    ///
    /// The 4 byte "Unique Disk ID" is used by recent Linux and Windows systems to identify the
    /// drive. "Unique" in this case means that the IDs of all the drives attached to a particular system are distinct.
    pub _id: u32,
    /// 0x1BC   2       Optional, reserved 0x0000
    ///
    /// The 2 byte reserved is usually 0x0000. 0x5A5A means read-only according to
    /// https://neosmart.net/wiki/mbr-boot-process/
    pub _reserved: u16,

    /// 0x1BE   16      First partition table entry
    /// 0x1CE   16      Second partition table entry
    /// 0x1DE   16      Third partition table entry
    /// 0x1EE   16      Fourth partition table entry
    pub entries: [PartitionTableEntry; 4],
    /// 0x1FE   2       (0x55, 0xAA) "Valid bootsector" signature bytes
    pub signature: u16,
}

impl PartitionTable {
    fn new(buf: &[u8]) -> PartitionTable {
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
            _bootstrap: bootstrap,
            _id: id,
            _reserved: reserved,
            entries,
            signature,
        }
    }
}

/// A partition.
struct Partition {
    block_idx: usize,
    start: BlockSector,
}

impl BlockOp for Partition {
    unsafe fn read(&mut self, sector: BlockSector, buf: &mut [u8]) -> Result<(), BlockError> {
        BLOCK_MANAGER
            .by_id(self.block_idx)
            .unwrap()
            .read(sector + self.start, buf)
    }

    unsafe fn write(&mut self, sector: BlockSector, buf: &[u8]) -> Result<(), BlockError> {
        BLOCK_MANAGER
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

pub fn partition_scan(block: &mut Block) {
    let mut part_nr = 0;
    read_partition_table(block, 0, 0, &mut part_nr);
    if part_nr == 0 {
        eprintln!("{}: Device contains no partitions", block.get_name());
    }
}

fn read_partition_table(
    block: &mut Block,
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
    block: &mut Block,
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
        unsafe {
            BLOCK_MANAGER.register_block(b_type, name.as_ref(), size, Box::new(p));
        }
    }
}
