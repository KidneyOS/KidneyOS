#![feature(allocator_api)]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(non_null_convenience)]
#![feature(slice_ptr_get)]
#![no_std]

pub mod global_descriptor_table;
pub mod macros;
pub mod mem;
pub mod paging;
pub mod segment;
pub mod serial;
pub mod sizes;
pub mod task_state_segment;
pub mod video_memory;
pub mod bit_array;
