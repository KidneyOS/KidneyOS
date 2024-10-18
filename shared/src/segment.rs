use arbitrary_int::{u13, u2};
use bitfield::bitfield;

bitfield!{
    pub struct SegmentSelector(u16);
    impl Debug;
    pub u2, requested_privilege_level, set_requested_privilege_level: 1, 0;
    pub descriptor_table, set_descriptor_table: 2;
    pub u13, index, set_index: 15, 3;
}
