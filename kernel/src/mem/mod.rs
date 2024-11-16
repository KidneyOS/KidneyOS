mod buddy_allocator;
mod dummy_allocator;
mod frame_allocator;
mod subblock_allocator;
pub mod user;
pub mod util;

use alloc::{boxed::Box, vec};
use core::sync::atomic::AtomicBool;
use core::{
    alloc::{AllocError, GlobalAlloc, Layout},
    cell::UnsafeCell,
    mem::size_of,
    ops::Range,
    ptr,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};
use dummy_allocator::DummyAllocatorSolution;
use frame_allocator::{CoreMapEntry, FrameAllocatorSolution};
use kidneyos_shared::{
    mem::{virt::trampoline_heap_top, BOOTSTRAP_ALLOCATOR_SIZE, OFFSET, PAGE_FRAME_SIZE},
    sizes::{KB, MB},
};
use subblock_allocator::SubblockAllocatorSolution;

static FIRST_ALLOCATION: AtomicBool = AtomicBool::new(true);
static TOTAL_NUM_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_NUM_DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

const MAX_SUPPORTED_ALIGN: usize = 4096;
/// "Upper memory" (as opposed to "lower memory") starts at 1MB.
const UPPER_MEMORY_START: usize = MB + OFFSET;

/// Function signature that all PlacementPolicies must follow
type PlacementPolicy = fn(
    core_map: &[CoreMapEntry],
    frames_requested: usize,
    _position: usize,
) -> Result<Range<usize>, AllocError>;

trait FrameAllocator {
    /// Allocates "frames_requested" number of contiguous frames
    ///
    /// This function should return a pointer to the start of the memory region on success and
    /// AllocError on failure
    fn alloc(&mut self, frames_requested: usize) -> Result<NonNull<[u8]>, AllocError>;

    /// Deallocates the frame or frames pointed to by "ptr_to_dealloc" according to layout
    ///
    /// This function should return the number of frames deallocated on success
    ///
    /// This function is unsafe because "ptr_to_dealloc" the caller must ensure that
    /// ptr_to_dealloc must be owned by the allocator
    unsafe fn dealloc(&mut self, ptr_to_dealloc: NonNull<u8>) -> usize;
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

enum KernelAllocatorState {
    DeInitialized,
    SetupState {
        dummy_allocator: DummyAllocatorSolution,
    },
    Initialized {
        subblock_allocator: SubblockAllocatorSolution,
    },
}

pub struct KernelAllocator {
    state: UnsafeCell<KernelAllocatorState>,
}

impl KernelAllocator {
    pub const fn new() -> KernelAllocator {
        Self {
            state: UnsafeCell::new(KernelAllocatorState::SetupState {
                dummy_allocator: DummyAllocatorSolution::new_in(0, 0),
            }),
        }
    }

    /// Initialize the kernel allocator
    ///
    /// "mem_upper" is the size of upper memory in kilobytes
    ///
    /// # Safety
    ///
    /// This function can only be called when the allocator is uninitialized.
    pub unsafe fn init(&mut self, mem_upper: usize) {
        let KernelAllocatorState::SetupState { dummy_allocator } = self.state.get_mut() else {
            // We can panic here because the kernel hasn't been initialized yet
            panic!("[PANIC]: init called while kernel allocator was already initialized");
        };

        // The exclusive max address is given by multiplying the number of bytes
        // in a KB by mem_upper, and adding this to UPPER_MEMORY_START.
        let frames_ceil_address = UPPER_MEMORY_START.saturating_add(mem_upper * KB);

        // TODO: Do we still need to add the BOOTSTRAP_ALLOCATOR_SIZE
        let frames_base_address = trampoline_heap_top() + BOOTSTRAP_ALLOCATOR_SIZE;

        // Check to see if dummy_allocator initialized properly (both start and end should be zero)
        let start = dummy_allocator.get_start_address();
        let end = dummy_allocator.get_end_address();
        assert_eq!(start, 0);
        assert_eq!(end, 0);

        // Set the proper start and end addresses
        dummy_allocator.set_start_address(frames_base_address);
        dummy_allocator.set_end_address(frames_ceil_address);

        let num_frames_in_system = (frames_ceil_address - frames_base_address)
            / (size_of::<CoreMapEntry>() + PAGE_FRAME_SIZE);

        // This should ALWAYS be the first global allocation to take place - should use dummy allocator
        //
        let core_map: Box<[CoreMapEntry]> =
            vec![CoreMapEntry::DEFAULT; num_frames_in_system].into_boxed_slice();

        // Check that the dummy allocator actually updated its internal state
        // I.e. the start address should have moved to accommodate Coremap Entries
        // The Coremap should take up 128 frames
        assert_ne!(frames_base_address, dummy_allocator.get_start_address());

        let frame_allocator = FrameAllocatorSolution::new(
            NonNull::new(dummy_allocator.get_start_address() as *mut u8)
                .expect("frames_base can't be null"),
            core_map,
        );

        *self.state.get_mut() = KernelAllocatorState::Initialized {
            subblock_allocator: SubblockAllocatorSolution::new(frame_allocator),
        };
    }

    pub fn frame_alloc(&mut self, frames: usize) -> Result<NonNull<[u8]>, AllocError> {
        let KernelAllocatorState::Initialized { subblock_allocator } = self.state.get_mut() else {
            return Err(AllocError);
        };

        subblock_allocator.get_frame_allocator().alloc(frames)
    }

    pub fn frame_dealloc(&mut self, ptr: NonNull<u8>) {
        let KernelAllocatorState::Initialized { subblock_allocator } = self.state.get_mut() else {
            halt!("[KERNEL ALLOCATOR]: Dealloc called on DeInitialized or SetupState kernel");
        };

        unsafe { subblock_allocator.get_frame_allocator().dealloc(ptr) };
    }

    pub fn deinit(&mut self) {
        let KernelAllocatorState::Initialized {
            subblock_allocator, ..
        } = self.state.get_mut()
        else {
            panic!("[KERNEL ALLOCATOR]: deinit called before initialization of kernel allocator");
        };

        let mut incorrect_num_allocs = false;

        if TOTAL_NUM_ALLOCATIONS.load(Ordering::Relaxed)
            != TOTAL_NUM_DEALLOCATIONS.load(Ordering::Relaxed)
        {
            incorrect_num_allocs = true;
        }

        subblock_allocator.deinit();

        if incorrect_num_allocs {
            halt!("[KERNEL ALLOCATOR]: Leaks detected");
        }

        *self.state.get_mut() = KernelAllocatorState::DeInitialized;
    }
}

// SAFETY:
//
// - We don't panic.
// - We don't mess up layout calculations.
// - We never rely on allocations happening.
unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if FIRST_ALLOCATION.load(Ordering::Relaxed) {
            // If we are here, it should be the dummy allocator doing the allocation
            let KernelAllocatorState::SetupState { dummy_allocator } = &mut *self.state.get()
            else {
                halt!("[KERNEL ALLOCATOR]: Kernel initialized before Coremap Entries created")
            };

            let size = layout.size();
            let align = layout.align();

            // The alignment of the layout should never be larger than the size of a page
            if align > MAX_SUPPORTED_ALIGN {
                return ptr::null_mut();
            }

            let num_frames_requested =
                ((size + align).next_multiple_of(PAGE_FRAME_SIZE)) / PAGE_FRAME_SIZE;

            let Ok(region) = dummy_allocator.alloc(num_frames_requested) else {
                halt!("[KERNEL ALLOCATOR]: Unable to allocate memory according to provided layout in DummyAllocator");
            };

            // We should never use dummy allocator again
            FIRST_ALLOCATION.store(false, Ordering::Relaxed);

            region.as_ptr().cast::<u8>()
        } else {
            let KernelAllocatorState::Initialized {
                subblock_allocator, ..
            } = &mut *self.state.get()
            else {
                halt!("[KERNEL ALLOCATOR]: Allocation requested before kernel is Initialized");
            };

            // The alignment of the layout should never be larger than the size of a page
            if layout.align() > MAX_SUPPORTED_ALIGN {
                return ptr::null_mut();
            }

            // Allocate using subblock allocator
            let ret_ptr = match subblock_allocator.allocate(layout) {
                Ok(t) => t,
                Err(_) => halt!("[KERNEL ALLOCATOR]: Unable to allocate memory according to provided layout in SubblockAllocator"),
            };

            TOTAL_NUM_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);

            ret_ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let KernelAllocatorState::Initialized {
            subblock_allocator, ..
        } = &mut *self.state.get()
        else {
            halt!("[KERNEL ALLOCATOR]: dealloc called before initialization of kernel allocator");
        };

        subblock_allocator.deallocate(ptr, layout);

        TOTAL_NUM_DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    }
}

// These tests do not use our global allocator
// TODO: Find a way to test the subblock/global allocator
#[cfg(test)]
mod test {
    use alloc::{boxed::Box, vec::Vec};

    #[test]
    fn test_box() {
        let heap_val_1 = Box::new(10);
        let heap_val_2 = Box::new(3.2);
        assert_eq!(*heap_val_1, 10);
        assert_eq!(*heap_val_2, 3.2);
    }

    #[test]
    fn test_vec() {
        let n = 20;
        let mut test_vec = Vec::new();
        for i in 1..=n {
            test_vec.push(i)
        }

        assert_eq!(test_vec[0], 1);
        assert_eq!(test_vec[10], 11);
        assert_eq!(test_vec.iter().sum::<u64>(), (n + 1) * (n / 2));
    }

    #[test]
    fn test_larger_vec() {
        let large_n = 60;
        let mut large_test_vec = Vec::new();
        for i in 1..=large_n {
            large_test_vec.push(i)
        }

        assert_eq!(large_test_vec[40], 41);
        assert_eq!(large_test_vec[52], 53);
        assert_eq!(
            large_test_vec.iter().sum::<u64>(),
            (large_n + 1) * (large_n / 2)
        );
    }
}
