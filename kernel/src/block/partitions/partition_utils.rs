#![allow(dead_code)] // Suppress unused warnings

use crate::block::block_core::BlockSector;

/// Converts a CHS address to an LBA address.
///
/// # Arguments
/// * `cylinder` - The cylinder number.
/// * `head` - The head number.
/// * `sector` - The sector number.
pub(crate) fn chs_to_lba(cylinder: u8, head: u8, sector: u8) -> BlockSector {
    (cylinder as u32 * 16 + head as u32) * 63 + sector as u32 - 1
}

/// Converts an LBA address to a CHS address.
///
/// # Arguments
/// * `lba` - The LBA address.
pub(crate) fn lba_to_chs(lba: BlockSector) -> (u8, u8, u8) {
    let sector = lba % 63 + 1;
    let temp = lba / 63;
    let head = temp % 16;
    let cylinder = temp / 16;

    (cylinder as u8, head as u8, sector as u8)
}

#[test]
fn test_chs_to_lba() {
    // Values taken from:
    // https://en.wikipedia.org/wiki/Logical_block_addressing#CHS_conversion

    assert_eq!(chs_to_lba(0, 0, 1), 0);
    assert_eq!(chs_to_lba(0, 0, 2), 1);
    assert_eq!(chs_to_lba(0, 0, 3), 2);
    assert_eq!(chs_to_lba(0, 0, 63), 62);
    assert_eq!(chs_to_lba(0, 1, 1), 63);
    assert_eq!(chs_to_lba(0, 15, 1), 945);
    assert_eq!(chs_to_lba(0, 15, 63), 1007);
    assert_eq!(chs_to_lba(1, 0, 1), 1008);
    assert_eq!(chs_to_lba(1, 0, 63), 1070);
    assert_eq!(chs_to_lba(1, 1, 1), 1071);
    assert_eq!(chs_to_lba(1, 1, 63), 1133);
    assert_eq!(chs_to_lba(1, 2, 1), 1134);
    assert_eq!(chs_to_lba(1, 15, 63), 2015);
    assert_eq!(chs_to_lba(2, 0, 1), 2016);
    assert_eq!(chs_to_lba(15, 15, 63), 16127);
    assert_eq!(chs_to_lba(16, 0, 1), 16128);
    assert_eq!(chs_to_lba(31, 15, 63), 32255);
    assert_eq!(chs_to_lba(32, 0, 1), 32256);
}

#[test]
fn test_lba_to_chs() {
    // Values taken from:
    // https://en.wikipedia.org/wiki/Logical_block_addressing#CHS_conversion

    assert_eq!(lba_to_chs(0), (0, 0, 1));
    assert_eq!(lba_to_chs(1), (0, 0, 2));
    assert_eq!(lba_to_chs(2), (0, 0, 3));
    assert_eq!(lba_to_chs(62), (0, 0, 63));
    assert_eq!(lba_to_chs(63), (0, 1, 1));
    assert_eq!(lba_to_chs(945), (0, 15, 1));
    assert_eq!(lba_to_chs(1007), (0, 15, 63));
    assert_eq!(lba_to_chs(1008), (1, 0, 1));
    assert_eq!(lba_to_chs(1070), (1, 0, 63));
    assert_eq!(lba_to_chs(1071), (1, 1, 1));
    assert_eq!(lba_to_chs(1133), (1, 1, 63));
    assert_eq!(lba_to_chs(1134), (1, 2, 1));
    assert_eq!(lba_to_chs(2015), (1, 15, 63));
    assert_eq!(lba_to_chs(2016), (2, 0, 1));
    assert_eq!(lba_to_chs(16127), (15, 15, 63));
    assert_eq!(lba_to_chs(16128), (16, 0, 1));
    assert_eq!(lba_to_chs(32255), (31, 15, 63));
    assert_eq!(lba_to_chs(32256), (32, 0, 1));
}
