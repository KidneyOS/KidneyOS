use core::usize;
use crate::user_program::time::get_tsc;

pub trait PageReplacementPolicy {
    fn evict_page(&mut self) -> usize;
}
