#![feature(allocator_api)]
#![cfg_attr(test, feature(btreemap_alloc))]
#![cfg_attr(target_os = "none", no_std)]

pub mod constants;
pub mod macros;
pub mod mem;
pub mod serial;
pub mod video_memory;
pub mod process;
pub mod pagedir;
pub mod syscall;
pub mod tss;

extern crate alloc;
