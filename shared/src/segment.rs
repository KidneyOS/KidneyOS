use crate::{bit_array::BitArray, bitfield};
use paste::paste;

bitfield!(
    SegmentSelector, u16
    {
        (u8, requested_privilege_level, 0, 1),
        (u16, index, 3, 15),
    }
    { (desciptor_table, 2) }
);

bitfield!(
    SegmentDescriptor, u64
    {
        (u16, limit_low, 0, 15),
        (u8, limit_high, 48, 51),
        (u16, base_low, 16, 31),
        (u8, base_mid, 32, 39),
        (u8, base_high, 56, 63),
        (u8, descriptor_privilege_level, 45, 46),
    }
    {
        (accessed, 40),
        (read_write, 41),
        (direction_conforming, 42),
        (executable, 43),
        (r#type, 44),
        (present, 47),
        (long_mode, 53),
        (size, 54),
        (granularity, 55),
    }
);

impl SegmentDescriptor {
    pub const fn limit(&self) -> u32 {
        ((self.limit_high() as u32) << 16) | (self.limit_low() as u32)
    }
    pub const fn with_limit(self, value: u32) -> Self {
        self.with_limit_low(value as u16)
            .with_limit_high((value >> 16) as u8)
    }

    pub const fn base(&self) -> u32 {
        ((self.base_high() as u32) << 24)
            | ((self.base_mid() as u32) << 16)
            | (self.base_low() as u32)
    }
    pub const fn with_base(self, value: u32) -> Self {
        self.with_base_low(value as u16)
            .with_base_mid((value >> 16) as u8)
            .with_base_high((value >> 24) as u8)
    }

    pub const UNLIMITED: Self = Self::default()
        .with_limit(0xFFFFF)
        .with_size(true)
        .with_granularity(true);
}
