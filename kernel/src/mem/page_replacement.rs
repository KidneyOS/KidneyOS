use core::usize;

pub trait PageReplacementPolicy {
    fn evict_page(&mut self) -> usize;
}
