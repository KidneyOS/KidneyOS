mod buddy_allocator;
mod frame_allocator;
mod pool_allocator;

use alloc::vec::Vec;
use buddy_allocator::BuddyAllocator;
use core::{
    alloc::{Allocator, GlobalAlloc, Layout},
    cell::UnsafeCell,
    ops::Range,
    ptr::NonNull,
};
use frame_allocator::FrameAllocatorSolution;
use kidneyos_core::{
    mem::{virt::trampoline_heap_top, OFFSET, PAGE_FRAME_SIZE},
    println,
    sizes::{KB, MB},
};

// Confirm that FrameAllocatorSolution has ::new_in and its result implements
// FrameAllocator.
fn __<A>(alloc: A) -> impl FrameAllocator<A>
where
    A: Allocator,
{
    FrameAllocatorSolution::<A>::new_in(alloc, 0)
}

/// # Safety
///
/// alloc must not return a range containing any frame index which has already
/// been returned by a prior alloc call and has not yet been deallocated.
unsafe trait FrameAllocator<A>
where
    A: Allocator,
{
    /// Create a new FrameAllocator.
    fn new_in(alloc: A, max_frames: usize) -> Self
    where
        A: Allocator,
        Self: Sized;

    /// Allocate the specified number of frames if possible, returning a range
    /// of indices for the allocated frames.
    fn alloc(&mut self, frames: usize) -> Option<Range<usize>>;

    /// Deallocate the previously allocated range of frames that begins at
    /// start.
    fn dealloc(&mut self, start: usize);
}

enum KernelAllocatorState {
    Uninitialized,
    Initialized {
        frame_allocator: FrameAllocatorSolution<BuddyAllocator>,
        frames_base: *mut u8,
        subblock_allocators: Vec<(BuddyAllocator, Range<usize>), BuddyAllocator>,
    },
}

pub struct KernelAllocator {
    state: UnsafeCell<KernelAllocatorState>,
}

impl KernelAllocator {
    pub const fn new() -> KernelAllocator {
        Self {
            state: UnsafeCell::new(KernelAllocatorState::Uninitialized),
        }
    }

    /// Initialize the kernel allocator. size is the size of kernel memory to
    /// prepare in bytes. mem_upper is the size of upper memory in kilobytes.
    /// Returns a pointer to the first
    ///
    /// # Safety
    ///
    /// This function can only be called when the allocator is uninitialized.
    pub unsafe fn init(&mut self, mem_upper: usize) {
        let KernelAllocatorState::Uninitialized = self.state.get_mut() else {
            panic!("init called while kernel allocator was already initialized");
        };

        // TODO: Check bounds with assertions.

        // TODO: We currently leave 8KB for the bootstrap allocator. This
        // should be re-evaluated later.
        const BUDDY_ALLOCATOR_SIZE: usize = 8 * KB;

        // "Upper memory" (as opposed to "lower memory") starts at 1MB.
        const UPPER_MEMORY_START: usize = MB + OFFSET;

        // The exclusive max address is given by multiplying the number of bytes
        // in a KB by mem_upper, and adding this to UPPER_MEMORY_START.
        let frames_max = UPPER_MEMORY_START.saturating_add(mem_upper * KB);

        // We start kernel virtual memory at the very end of upper memory, so
        // the start address is the max address minus the size.
        let bootstrap_base = trampoline_heap_top() as *mut u8;

        let bootstrap_allocator = BuddyAllocator::new(NonNull::slice_from_raw_parts(
            NonNull::new_unchecked(bootstrap_base),
            BUDDY_ALLOCATOR_SIZE,
        ));

        let frames_base = bootstrap_base.add(BUDDY_ALLOCATOR_SIZE);
        let max_frames = (frames_max - frames_base as usize) / PAGE_FRAME_SIZE;
        *self.state.get_mut() = KernelAllocatorState::Initialized {
            frame_allocator: FrameAllocatorSolution::new_in(bootstrap_allocator, max_frames),
            frames_base,
            subblock_allocators: Vec::new_in(bootstrap_allocator),
        };
    }

    /// Deinitialize the kernel allocator, printing information about any leaks
    /// that have occurred. panics if any leaks are found.
    ///
    /// # Safety
    ///
    /// This function can only be called when the allocator is initialized.
    pub unsafe fn deinit(&mut self) {
        let KernelAllocatorState::Initialized {
            subblock_allocators,
            ..
        } = self.state.get_mut()
        else {
            panic!("deinit called before initialization of kernel allocator");
        };

        let mut leaked = false;
        for (subblock_allocator, _) in subblock_allocators.iter() {
            leaked |= subblock_allocator.detect_leaks();
        }

        assert!(leaked || subblock_allocators.is_empty());

        // We can't sucessfully deinitialize because there are still references
        // to the memory that we would loose by deinitializing.
        if leaked {
            println!();
            panic!("leaks detected");
        }

        *self.state.get_mut() = KernelAllocatorState::Uninitialized;
    }
}

// halt is used for cases where we would panic in KernelAllocator, but can't
// because doing so causes undefined behaviour as per the GlobalAlloc safety
// contract.
macro_rules! halt {
    () => {{
        super::eprintln!();
        loop {}
    }};
    ($($arg:tt)*) => {{
        kidneyos_core::eprintln!($($arg)*);
        loop {}
    }};
}

// SAFETY:
//
// - We don't panic.
// - We don't mess up layout calculations.
// - We never rely on allocations happening.
unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let KernelAllocatorState::Initialized {
            frame_allocator,
            frames_base,
            subblock_allocators,
        } = &mut *self.state.get()
        else {
            halt!("alloc called before initialization of kernel allocator");
        };

        // First see if we have space in any of our existing subblock
        // allocators, and if so return memory from there.
        for (subblock_allocator, _) in subblock_allocators.iter() {
            if let Ok(res) = subblock_allocator.allocate(layout) {
                return res.as_ptr().cast::<u8>();
            }
        }

        let Some(range) = frame_allocator.alloc(
            (layout.size() + layout.align() - 1 + BuddyAllocator::OVERHEAD)
                .next_multiple_of(PAGE_FRAME_SIZE)
                / PAGE_FRAME_SIZE,
        ) else {
            halt!("Out of virtual memory!");
        };

        let region = NonNull::slice_from_raw_parts(
            NonNull::new_unchecked(frames_base.add(range.start * PAGE_FRAME_SIZE)),
            range.len() * PAGE_FRAME_SIZE,
        );
        let buddy_allocator = BuddyAllocator::new(region);
        subblock_allocators.push((buddy_allocator, range));
        buddy_allocator
            .allocate(layout)
            .expect("new buddy allocator created with sufficient region failed to fit planned allocation")
            .as_ptr()
            .cast::<u8>()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let KernelAllocatorState::Initialized {
            frame_allocator,
            frames_base,
            subblock_allocators,
        } = &mut *self.state.get()
        else {
            halt!("dealloc called before initialization of kernel allocator");
        };

        let frame_index = (ptr as usize - *frames_base as usize) / PAGE_FRAME_SIZE;

        // Scope ensures we drop subblock_allocator (which should be the only
        // reference to it, or its memory) before dealloc'ing its backing frames
        // out from under it.
        let (at, start) = {
            let Some((at, (subblock_allocator, range))) = subblock_allocators
                .iter()
                .enumerate()
                .find(|(_, (_, range))| range.contains(&frame_index))
            else {
                halt!("internal inconsistency detected in kernel allocator")
            };

            subblock_allocator.deallocate(NonNull::new_unchecked(ptr), layout);

            if !subblock_allocator.is_empty() {
                return;
            }

            (at, range.start)
        };

        subblock_allocators.remove(at);
        frame_allocator.dealloc(start);
    }
}
