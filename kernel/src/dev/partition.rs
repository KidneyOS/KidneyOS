#![allow(unused_variables)]
#![allow(dead_code)]


use super::block::{BlockSector, BlockType, BlockManager, Block};
use alloc::string::{String, ToString};
use core::mem::transmute;
use core::clone::Clone;
use kidneyos_shared::println;


#[repr(packed, C)]
struct Pte {
    bootable: u8,
    start_chs: [u8; 3],
    ptype: u8,
    end_chs:[u8; 3],
    offset: BlockSector,
    size: BlockSector,
}
#[repr(packed, C)]
struct PartitionTable{
    loader: [u8; 446],
    partitions: [Pte; 4],
    signature: u16, /* Should be 0xaa55 */
}


pub fn partition_scan (idx: usize, mut all_blocks: BlockManager) -> BlockManager {
    let mut pn = 0;
    all_blocks = read_partition_table(all_blocks.by_id(idx), 0, 0, &mut pn, all_blocks);
    if pn == 0 {
        println!("{}: Device contains no partitions", all_blocks.by_id(idx).block_name());
    }
    all_blocks
}


// Read MBR Partition table and register block devices
fn read_partition_table(
    dev: Block,
    sector: BlockSector,
    primary_extended_sector: BlockSector,
    pn: &mut usize,
    mut all_blocks: BlockManager,
) -> BlockManager {

    assert!(dev.block_type() == BlockType::Raw);
    let mut pt_rb: [u8; 512] = [0; 512];

    if sector > dev.block_size() {
        println!("{}: Partition at sector {} past end of device", dev.block_name(), sector);
        return all_blocks;
    }
    
    dev.block_read(0, &mut pt_rb);
    let pt: PartitionTable;
    unsafe {
        pt  = transmute(pt_rb);
    };

    if pt.signature != 0xaa55 {
        if primary_extended_sector == 0 {
            println!("{}: Invalid PTE Signiture", dev.block_name());
        } else {
            println!("{}: Invalide extended partition table in sector {}", dev.block_name(), sector);
        }
        return all_blocks;
    }

    for e in pt.partitions {
        if (e.size as usize)== 0 || e.ptype == 0 {
            
        } else if e.ptype == 0x05 
                || e.ptype == 0x0f 
                || e.ptype == 0x85
                || e.ptype == 0xc5
        {
            println!("{}: Extended partition in sector {}", dev.block_name(), sector);
            if sector == 0 {
                all_blocks = read_partition_table (dev.clone(), e.offset, e.offset, pn, all_blocks);
            }
            else {
                all_blocks = read_partition_table (dev.clone(), e.offset + primary_extended_sector, primary_extended_sector, pn, all_blocks);
            }
        } else {
            *pn += 1; 
            all_blocks = found_partition (dev.clone(), e.ptype, e.offset + sector, e.size, *pn, all_blocks);
        }
    }
    all_blocks
}


fn found_partition(
    dev: Block,
    ptype: u8,
    start: BlockSector,
    size: BlockSector,
    part_no: usize,
    mut all_blocks:  BlockManager,
) -> BlockManager {
    if start  + size > dev.block_size() {
        println! ("{}{}: Partition ends after device", dev.block_name(), part_no);
        return all_blocks;
    }
    
    let ptype: BlockType = match ptype {
        0x20 => BlockType::Kernel(start), 
        0x21 => BlockType::Filesys(start),
        0x22 => BlockType::Scratch(start),
        0x23 => BlockType::Swap(start),
        _    => BlockType::Foreign(start)
    };
    let mut name: String = dev.block_name().into();
    name.push_str(part_no.to_string().as_str());   

    println!("found partition: {} start: {}, size: {}M", name, start, size >> 11);
     
    all_blocks.block_register(
        ptype,
        name,
        size,
        dev.driver(),
    );
    all_blocks
}

// static fn type_lookup(type: u8) {
//     match type {
//         0x00 => "Empty",
//         0x01 => "FAT12",
//         0x02 => "XENIX root",
//         0x03 => "XENIX usr",
//         0x04 => "FAT16 <32M",
//         0x05 => "Extended",
//         0x06 => "FAT16",
//         0x07 => "HPFS/NTFS",
//         0x08 => "AIX",
//         0x09 => "AIX bootable",
//         0x0a => "OS/2 Boot Manager",
//         0x0b => "W95 FAT32",
//         0x0c => "W95 FAT32 (LBA)",
//         0x0e => "W95 FAT16 (LBA)",
//         0x0f => "W95 Ext'd (LBA)",
//         0x10 => "OPUS",
//         0x11 => "Hidden FAT12",
//         0x12 => "Compaq diagnostics",
//         0x14 => "Hidden FAT16 <32M",
//         0x16 => "Hidden FAT16",
//         0x17 => "Hidden HPFS/NTFS",
//         0x18 => "AST SmartSleep",
//         0x1b => "Hidden W95 FAT32",
//         0x1c => "Hidden W95 FAT32 (LBA)",
//         0x1e => "Hidden W95 FAT16 (LBA)",
//         0x20 => "Pintos OS kernel",
//         0x21 => "Pintos file system",
//         0x22 => "Pintos scratch",
//         0x23 => "Pintos swap",
//         0x24 => "NEC DOS",
//         0x39 => "Plan 9",
//         0x3c => "PartitionMagic recovery",
//         0x40 => "Venix 80286",
//         0x41 => "PPC PReP Boot",
//         0x42 => "SFS",
//         0x4d => "QNX4.x",
//         0x4e => "QNX4.x 2nd part",
//         0x4f => "QNX4.x 3rd part",
//         0x50 => "OnTrack DM",
//         0x51 => "OnTrack DM6 Aux1",
//         0x52 => "CP/M",
//         0x53 => "OnTrack DM6 Aux3",
//         0x54 => "OnTrackDM6",
//         0x55 => "EZ-Drive",
//         0x56 => "Golden Bow",
//         0x5c => "Priam Edisk",
//         0x61 => "SpeedStor",
//         0x63 => "GNU HURD or SysV",
//         0x64 => "Novell Netware 286",
//         0x65 => "Novell Netware 386",
//         0x70 => "DiskSecure Multi-Boot",
//         0x75 => "PC/IX",
//         0x80 => "Old Minix",
//         0x81 => "Minix / old Linux",
//         0x82 => "Linux swap / Solaris",
//         0x83 => "Linux",
//         0x84 => "OS/2 hidden C: drive",
//         0x85 => "Linux extended",
//         0x86 => "NTFS volume set",
//         0x87 => "NTFS volume set",
//         0x88 => "Linux plaintext",
//         0x8e => "Linux LVM",
//         0x93 => "Amoeba",
//         0x94 => "Amoeba BBT",
//         0x9f => "BSD/OS",
//         0xa0 => "IBM Thinkpad hibernation",
//         0xa5 => "FreeBSD",
//         0xa6 => "OpenBSD",
//         0xa7 => "NeXTSTEP",
//         0xa8 => "Darwin UFS",
//         0xa9 => "NetBSD",
//         0xab => "Darwin boot",
//         0xb7 => "BSDI fs",
//         0xb8 => "BSDI swap",
//         0xbb => "Boot Wizard hidden",
//         0xbe => "Solaris boot",
//         0xbf => "Solaris",
//         0xc1 => "DRDOS/sec (FAT-12)",
//         0xc4 => "DRDOS/sec (FAT-16 < 32M)",
//         0xc6 => "DRDOS/sec (FAT-16)",
//         0xc7 => "Syrinx",
//         0xda => "Non-FS data",
//         0xdb => "CP/M / CTOS / ...",
//         0xde => "Dell Utility",
//         0xdf => "BootIt",
//         0xe1 => "DOS access",
//         0xe3 => "DOS R/O",
//         0xe4 => "SpeedStor",
//         0xeb => "BeOS fs",
//         0xee => "EFI GPT",
//         0xef => "EFI (FAT-12/16/32)",
//         0xf0 => "Linux/PA-RISC boot",
//         0xf1 => "SpeedStor",
//         0xf4 => "SpeedStor",
//         0xf2 => "DOS secondary",
//         0xfd => "Linux raid autodetect",
//         0xfe => "LANstep",
//         0xff => "BBT",
//     }
// }


