use arbitrary_int::{u13, u2};
use bitbybit::bitfield;

#[bitfield(u16, default = 0)]
pub struct SegmentSelector {
    #[bits(0..=1, rw)]
    requested_privilege_level: u2,
    #[bit(2, rw)]
    descriptor_table: bool,
    #[bits(3..=15, rw)]
    index: u13,
}
