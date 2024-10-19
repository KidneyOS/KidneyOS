mod buddy_allocator;
mod frame_allocator;
pub mod user;
pub mod util;
mod swapping;
mod page_replacement;

use alloc::vec::Vec;
use buddy_allocator::BuddyAllocator;
use core::{
    alloc::{AllocError, Allocator, GlobalAlloc, Layout},
    cell::UnsafeCell,
    ops::Range,
    ptr::NonNull,
};
use frame_allocator::FrameAllocatorSolution;
use kidneyos_shared::{
    mem::{virt::trampoline_heap_top, BOOTSTRAP_ALLOCATOR_SIZE, OFFSET, PAGE_FRAME_SIZE},
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

struct FrameAllocatorWrapper<A: Allocator> {
    start: NonNull<u8>,
    frame_allocator: FrameAllocatorSolution<A>,
}

impl<A: Allocator> FrameAllocatorWrapper<A> {
    fn new_in(alloc: A, start: NonNull<u8>, max_frames: usize) -> Self {
        Self {
            start,
            frame_allocator: FrameAllocatorSolution::new_in(alloc, max_frames),
        }
    }

    pub fn alloc(&mut self, frames: usize) -> Result<NonNull<[u8]>, AllocError> {
        let Some(range) = self.frame_allocator.alloc(frames) else {
            return Err(AllocError);
        };

        Ok(NonNull::slice_from_raw_parts(
            NonNull::new(unsafe { self.start.as_ptr().add(range.start * PAGE_FRAME_SIZE) })
                .ok_or(AllocError)?,
            range.len() * PAGE_FRAME_SIZE,
        ))
    }

    pub fn dealloc(&mut self, ptr: NonNull<u8>) {
        let start = (ptr.as_ptr() as usize - self.start.as_ptr() as usize) / PAGE_FRAME_SIZE;
        self.frame_allocator.dealloc(start);
    }
}

enum KernelAllocatorState {
    Uninitialized,
    Initialized {
        frame_allocator: FrameAllocatorWrapper<BuddyAllocator>,
        subblock_allocators: Vec<(BuddyAllocator, NonNull<[u8]>), BuddyAllocator>,
    },
}

pub struct KernelAllocator {
    state: UnsafeCell<KernelAllocatorState>,
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
        kidneyos_shared::eprintln!($($arg)*);
        loop {}
    }};
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

        // "Upper memory" (as opposed to "lower memory") starts at 1MB.
        const UPPER_MEMORY_START: usize = MB + OFFSET;

        // The exclusive max address is given by multiplying the number of bytes
        // in a KB by mem_upper, and adding this to UPPER_MEMORY_START.
        let frames_max = UPPER_MEMORY_START.saturating_add(mem_upper * KB);

        let bootstrap_base = trampoline_heap_top() as *mut u8;

        let bootstrap_allocator = BuddyAllocator::new(NonNull::slice_from_raw_parts(
            NonNull::new_unchecked(bootstrap_base),
            BOOTSTRAP_ALLOCATOR_SIZE,
        ));

        let frames_base = bootstrap_base.add(BOOTSTRAP_ALLOCATOR_SIZE).cast::<u8>();
        let max_frames = (frames_max - frames_base as usize) / PAGE_FRAME_SIZE;
        *self.state.get_mut() = KernelAllocatorState::Initialized {
            frame_allocator: FrameAllocatorWrapper::new_in(
                bootstrap_allocator,
                NonNull::new(frames_base).expect("frames_base can't be null"),
                max_frames,
            ),
            subblock_allocators: Vec::new_in(bootstrap_allocator),
        };
    }

    /// # Safety
    ///
    /// TODO
    pub unsafe fn frame_alloc(&mut self, frames: usize) -> Result<NonNull<[u8]>, AllocError> {
        let KernelAllocatorState::Initialized {
            frame_allocator, ..
        } = &mut *self.state.get()
        else {
            return Err(AllocError);
        };

        frame_allocator.alloc(frames)
    }

    /// # Safety
    ///
    /// TODO
    pub unsafe fn frame_dealloc(&mut self, ptr: NonNull<u8>) {
        let KernelAllocatorState::Initialized {
            frame_allocator, ..
        } = &mut *self.state.get()
        else {
            halt!("dealloc called before initialization of kernel allocator");
        };

        frame_allocator.dealloc(ptr)
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

// SAFETY:
//
// - We don't panic.
// - We don't mess up layout calculations.
// - We never rely on allocations happening.
unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let KernelAllocatorState::Initialized {
            frame_allocator,
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

        let Ok(region) = frame_allocator.alloc(
            (layout.size() + layout.align() - 1 + BuddyAllocator::OVERHEAD)
                .next_multiple_of(PAGE_FRAME_SIZE)
                / PAGE_FRAME_SIZE,
        ) else {
            // Evict page for swapping

            halt!("Out of virtual memory!");
        };

        let buddy_allocator = BuddyAllocator::new(region);
        subblock_allocators.push((buddy_allocator, region));
        buddy_allocator
            .allocate(layout)
            .expect("new buddy allocator created with sufficient region failed to fit planned allocation")
            .as_ptr()
            .cast::<u8>()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let KernelAllocatorState::Initialized {
            frame_allocator,
            subblock_allocators,
        } = &mut *self.state.get()
        else {
            halt!("dealloc called before initialization of kernel allocator");
        };

        // Scope ensures we drop subblock_allocator (which should be the only
        // reference to it, or its memory) before dealloc'ing its backing frames
        // out from under it.
        let (at, ptr) = {
            let Some((at, (subblock_allocator, region))) = subblock_allocators
                .iter()
                .enumerate()
                .find(|(_, (_, region))| {
                    let start = region.as_ptr().cast::<u8>();
                    start <= ptr && ptr < start.add(region.len())
                })
            else {
                halt!(
                    "internal inconsistency detected in kernel allocator with ptr {:#X}",
                    ptr as usize
                )
            };

            subblock_allocator.deallocate(NonNull::new_unchecked(ptr), layout);

            if !subblock_allocator.is_empty() {
                return;
            }

            (at, region.cast::<u8>())
        };

        subblock_allocators.remove(at);
        frame_allocator.dealloc(ptr);
    }
}
