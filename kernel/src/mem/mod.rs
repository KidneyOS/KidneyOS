#![feature(new_uninit)]
mod buddy_allocator;
mod frame_allocator;
pub mod user;
pub mod util;

use alloc::vec::Vec;
use buddy_allocator::BuddyAllocator;
use core::{
    alloc::{AllocError, Allocator, GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::NonNull,
    mem::size_of,
};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering};
use frame_allocator::{CoreMapEntry, FrameAllocatorSolution, DummyAllocatorSolution};
use kidneyos_shared::{
    mem::{virt::trampoline_heap_top, BOOTSTRAP_ALLOCATOR_SIZE, OFFSET, PAGE_FRAME_SIZE},
    println,
    sizes::{KB, MB},
};


// Global variables to keep track of allocation statistics
static TOTAL_NUM_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_NUM_DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_NUM_FRAMES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

// The alignment of the layout cannot be greater than the size of the page
const MAX_SUPPORTED_ALIGN: usize = 4096;


// Confirm that FrameAllocatorSolution has ::new_in and its result implements
// FrameAllocator.
fn __(start: NonNull<u8>,
      core_map: Box<[CoreMapEntry]>,
      num_frames_in_system: usize) -> impl FrameAllocator{
    FrameAllocatorSolution::new_in(start, core_map, num_frames_in_system);
}

unsafe trait FrameAllocator
{
    /// Create a new FrameAllocator.
    fn new_in(start: NonNull<u8>,
              core_map: Box<[CoreMapEntry]>,
              num_frames_in_system: usize) -> Self;

    /// Allocate the specified number of frames if possible,
    /// Input: The numbers of frames wanted
    /// Output: Pointer to piece of memory satisfying requirements or AllocError if not enough
    /// room available
    fn alloc(&mut self, frames_requested: usize) -> Result<NonNull<[u8]>, AllocError>;

    /// Deallocate the previously allocated range of frames that begins at start.
    /// Input: Pointer to region of memory to be deallocated
    fn dealloc(&mut self, ptr_to_dealloc: NonNull<u8>);
}

struct FrameAllocatorWrapper{
    frame_allocator: FrameAllocatorSolution,
}

impl FrameAllocatorWrapper{
    fn new_in(start: NonNull<u8>, core_map: Box<[CoreMapEntry]>, num_frames_in_system: usize) -> Self {
        Self {
            frame_allocator: FrameAllocatorSolution::new_in(start: NonNull<u8>,
                                                            core_map,
                                                            num_frames_in_system),
        }
    }

    pub fn alloc(&mut self, frames: usize) -> Result<NonNull<[u8]>, AllocError> {
        self.frame_allocator.alloc(frames)
    }

    pub fn dealloc(&mut self, ptr: NonNull<u8>) {
        self.frame_allocator.dealloc(ptr);
    }
}

enum KernelAllocatorState {
    Uninitialized {
        dummy_allocator: DummyAllocatorSolution,
    },
    Initialized {
        frame_allocator: FrameAllocatorWrapper,
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
            state: UnsafeCell::new(KernelAllocatorState::Uninitialized{
                dummy_allocator: DummyAllocatorSolution::new_in(0, 0), }),
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
        let KernelAllocatorState::Uninitialized {
            dummy_allocator
        } = &mut *self.state.get_mut() else {
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
        let frames_base_pointer = bootstrap_base.add(BOOTSTRAP_ALLOCATOR_SIZE).cast::<u8>();

        // Now that we know the start and end bounds for our memory region, we can set it in the
        // dummy; initial values should be 0 if set correctly
        let start = dummy_allocator.get_start_address();
        let end = dummy_allocator.get_end_address();
        assert_eq!(start, 0);
        assert_eq!(end, 0);

        dummy_allocator.set_start_address(frames_base_pointer as usize);
        dummy_allocator.set_end_address(frames_max);

        let num_frames_in_system = (frames_max - frames_base_pointer as usize) /
            (size_of::<CoreMapEntry>() + PAGE_FRAME_SIZE);

        // This should ALWAYS be the first global allocation to take place - should use dummy allocator
        let mut core_map: Box<[CoreMapEntry]> = vec![CoreMapEntry::DEFAULT; num_frames_in_system]
                                                .into_boxed_slice();

        // With the core_map not initialized, we can now initialize the actual Frame Allocator
        *self.state.get_mut() = KernelAllocatorState::Initialized {
            frame_allocator: FrameAllocatorWrapper::new_in(
                NonNull::new(dummy_allocator.get_start_address() as *mut u8).expect("frames_base can't be null"),
                core_map,
                num_frames_in_system,
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

        // We can't successfully deinitialize because there are still references
        // to the memory that we would lose by deinitializing.
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
        if TOTAL_NUM_ALLOCATIONS.load(Ordering::Relaxed) == 0 {
            /*
            TODO: Add has_room/has_space function in frame allocator for future implementation?
            1. Check that kernel is in Uninitialized state, panic if not
            2. Calculate the number of frames that have been requested in the layout
            3. Call the dummy allocator alloc
            4. Increment global statistics

            TODO: Maybe change this?
            If there is not enough room, just PANIC!!!
             */
            let KernelAllocatorState::Uninitialized {
                dummy_allocator
            } = &mut *self.state.get() else {
                halt!("Kernel initialized before Coremap entries were setup, abort")
            };

            let size = layout.size();
            let align = layout.align();

            // The alignment of the layout should never be larger than the size of a page
            if align > MAX_SUPPORTED_ALIGN{
                return null_mut();
            }

            let num_frames_requested = ((size + align).next_multiple_of(PAGE_FRAME_SIZE))
                                                        / PAGE_FRAME_SIZE;

            let Ok(region) = dummy_allocator.alloc(num_frames_requested) else {
                halt!("Unable to allocate memory according to provided layout, PANIC!");
            };

            // At this point, we know the allocation was successful; increment global statistics
            let new_total_allocs = TOTAL_NUM_ALLOCATIONS.load(Ordering::Relaxed) + 1;
            TOTAL_NUM_ALLOCATIONS.store(new_total_allocs, Ordering::Relaxed);
            let new_total_frames = TOTAL_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + num_frames_requested;
            TOTAL_NUM_FRAMES_ALLOCATED.store(new_total_frames, Ordering::Relaxed);

            region.as_ptr().cast::<u8>()
        } else {
            // TODO: Change this later once subblock allocator is updated
            let KernelAllocatorState::Initialized {
                frame_allocator,
                subblock_allocators: _subblock_allocators,
            } = &mut *self.state.get()
                else {
                    halt!("alloc called before initialization of kernel allocator");
                };

            let size = layout.size();
            let align = layout.align();

            // The alignment of the layout should never be larger than the size of a page
            if align > MAX_SUPPORTED_ALIGN{
                return null_mut();
            }

            let num_frames_requested = ((size + align).next_multiple_of(PAGE_FRAME_SIZE))
                / PAGE_FRAME_SIZE;

            let Ok(region) = frame_allocator.alloc(num_frames_requested) else {
                halt!("Unable to allocate memory according to provided layout, PANIC!");
            };

            // At this point, we know the allocation was successful; increment global statistics
            let new_total_allocs = TOTAL_NUM_ALLOCATIONS.load(Ordering::Relaxed) + 1;
            TOTAL_NUM_ALLOCATIONS.store(new_total_allocs, Ordering::Relaxed);
            let new_total_frames = TOTAL_NUM_FRAMES_ALLOCATED.load(Ordering::Relaxed) + num_frames_requested;
            TOTAL_NUM_FRAMES_ALLOCATED.store(new_total_frames, Ordering::Relaxed);

            region.as_ptr().cast::<u8>()
        }
    }

    // TODO: Implement dealloc later
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
