pub mod pool_allocator;
pub mod mem_addr_types;

use crate::sizes::{KB, MB};

// Page size is 4KB. This is a property of x86 processors.
pub const PAGE_FRAME_SIZE: usize = 4 * KB;
pub const HUGE_PAGE_SIZE: usize = 4 * MB;

macro_rules! linker_offsets {
    ($($name:ident),*) => {
        $(
        #[inline]
        pub fn $name() -> usize {
            extern "C" {
                static $name: u8;
            }

            // SAFETY: The linker script will give this the correct address.
            unsafe { core::ptr::addr_of!($name) as usize }
        }
        )*
    }
}

pub mod phys {
    linker_offsets!(trampoline_start, trampoline_data_start, trampoline_end);

    macro_rules! to_phys {
        ($($name:ident),*) => {
            $(
            #[inline]
            pub fn $name() -> usize {
                super::virt::$name() - super::OFFSET
            }
            )*
        }
    }

    to_phys!(kernel_start, kernel_data_start, kernel_end);

    #[inline]
    pub fn main_stack_top() -> usize {
        kernel_end() + super::MAIN_STACK_SIZE
    }

    #[inline]
    pub fn trampoline_heap_top() -> usize {
        main_stack_top() + super::TRAMPOLINE_HEAP_SIZE
    }
}

pub mod virt {
    linker_offsets!(kernel_start, kernel_data_start, kernel_end);

    macro_rules! to_virt {
        ($($name:ident),*) => {
            $(
            #[inline]
            pub fn $name() -> usize {
                super::phys::$name() + super::OFFSET
            }
            )*
        }
    }

    to_virt!(main_stack_top, trampoline_heap_top);
}

// Any virtual address at or above OFFSET is a kernel address.
pub const OFFSET: usize = 0x80000000;

// TODO: Figure out how to detect kernel stack overflows.
pub const MAIN_STACK_SIZE: usize = 32 * KB;
pub const TRAMPOLINE_HEAP_SIZE: usize = 8 * MB;

// TODO: We currently leave 8MB for the bootstrap allocator. This should be
// re-evaluated later.
pub const BOOTSTRAP_ALLOCATOR_SIZE: usize = 8 * MB;
