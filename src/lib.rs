#![feature(allocator_api)]
#![feature(naked_functions)]
#![cfg_attr(test, feature(btreemap_alloc))]
#![cfg_attr(target_os = "none", no_std)]

pub mod constants;
pub mod macros;
pub mod mem;
pub mod serial;
pub mod threading;
pub mod video_memory;

extern crate alloc;
