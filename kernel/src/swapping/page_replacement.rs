use core::usize;
use kidneyos_shared::println;

use crate::user_program::time::get_tsc;

pub trait PageReplacementPolicy {
    fn evict_page(max_frames: usize) -> usize;
}

pub struct RandomEviction {}

impl PageReplacementPolicy for RandomEviction {
    fn evict_page(max_frames: usize) -> usize {
        let time = get_tsc().tv_sec;

        // println!("{}", time as usize % max_frames);
        return time as usize % max_frames;
    }
}
