#[derive(Clone, Copy, Debug)]
pub struct BitArray<T>(pub T);

macro_rules! impl_bitarray {
    ({ $($t: ty),* } { $($v: ty),* }) => {
        $(
            impl BitArray<$t> {
                pub const fn get(&self, idx: usize) -> bool {
                    (self.0 >> idx & 1) != 0
                }
                pub const fn get_range(&self, low: usize, high: usize) -> $v {
                    let mask = ((1 << high - low) - 1 | 1 << high - low) << low;
                    (self.0 & mask) >> low as $v
                }

                pub const fn with(self, value: bool, idx: usize) -> Self {
                    BitArray((self.0 & !(1 << idx)) | (value as $t) << idx)
                }

                pub const fn with_range(self, value: $v, low: usize, high: usize) -> Self {
                    let mask = ((1 << high - low) - 1 | 1 << high - low) << low;
                    BitArray((self.0 & !mask) | (value as $t << low) & mask)
                }

                pub const fn load(self) -> $t { self.0 }
            }
        )*
    };
}

impl_bitarray!({u8, u16, u32, u64} {u8, u16, u32, u64});

#[macro_export]
macro_rules! bitfield {
    (
        $name: ident,
        $t: ty
        { $( ($r_t: ty, $r_name: ident, $r_low: literal, $r_high: literal) ),* $(,)? }
        { $( ($b_name: ident, $b_idx: literal) ),* $(,)? }
    ) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $name(BitArray<$t>);

        impl $name {
            #[allow(dead_code)]
            pub const fn default() -> Self { Self(BitArray::<$t>(0)) }
            #[allow(dead_code)]
            pub const fn new(value: $t) -> Self { Self(BitArray::<$t>(value)) }
            #[allow(dead_code)]
            pub const fn load(self) -> $t { self.0.load() }

            $(
                paste! {
                    #[allow(dead_code)]
                    pub const fn $r_name(&self) -> $r_t { self.0.get_range($r_low, $r_high) as $r_t }
                    #[allow(dead_code)]
                    pub const fn [<with_ $r_name>](self, value: $r_t) -> Self {
                        Self(self.0.with_range(value as $t, $r_low, $r_high))
                    }
                }
            )*

            $(
                paste! {
                    #[allow(dead_code)]
                    pub const fn $b_name(&self) -> bool { self.0.get($b_idx) }
                    #[allow(dead_code)]
                    pub const fn [<with_ $b_name>](self, value: bool) -> Self {
                        Self(self.0.with(value, $b_idx))
                    }
                }
            )*
        }
    };
}
