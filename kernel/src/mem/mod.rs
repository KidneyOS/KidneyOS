mod buddy_allocator;
mod frame_allocator;
pub mod user;
pub mod util;
mod subblock_allocator;
mod subblock_allocator_new;

use alloc::{
    boxed::Box,
    vec::Vec,

};
use core::{
    alloc::{AllocError, GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::NonNull,
    mem::size_of,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};
use frame_allocator::{CoreMapEntry, FrameAllocatorSolution, DummyAllocatorSolution};
use subblock_allocator_new::SubblockAllocator;
use kidneyos_shared::{
    mem::{virt::trampoline_heap_top, BOOTSTRAP_ALLOCATOR_SIZE, OFFSET, PAGE_FRAME_SIZE},
    println,
    sizes::{KB, MB},
};


static TOTAL_NUM_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_NUM_DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

const MAX_SUPPORTED_ALIGN: usize = 4096;
/// "Upper memory" (as opposed to "lower memory") starts at 1MB.
const UPPER_MEMORY_START: usize = MB + OFFSET;

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
    /// Output: The number of frames deallocated
    fn dealloc(&mut self, ptr_to_dealloc: NonNull<u8>) -> usize;
}

struct FrameAllocatorWrapper{
    frame_allocator: FrameAllocatorSolution,
}

impl FrameAllocatorWrapper{
    fn new_in(start: NonNull<u8>, core_map: Box<[CoreMapEntry]>, num_frames_in_system: usize) -> Self {
        Self {
            frame_allocator: FrameAllocatorSolution::new_in(start,
                                                            core_map,
                                                            num_frames_in_system),
        }
    }

    pub fn alloc(&mut self, frames: usize) -> Result<NonNull<[u8]>, AllocError> {
        self.frame_allocator.alloc(frames)
    }

    pub fn dealloc(&mut self, ptr: NonNull<u8>) -> usize{
        self.frame_allocator.dealloc(ptr)
    }
}

enum KernelAllocatorState {
    DeInitialized,
    SetupState {
        dummy_allocator: DummyAllocatorSolution,
    },
    Initialized {
        subblock_allocator: SubblockAllocator,
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
            state: UnsafeCell::new(
                KernelAllocatorState::SetupState {
                    dummy_allocator: DummyAllocatorSolution::new_in(0, 0)
                }
            )
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
        let KernelAllocatorState::SetupState {
            dummy_allocator
        } = &mut *self.state.get_mut() else {
            // We can panic here because the kernel hasn't been initialized yet
            panic!("[PANIC]: init called while kernel allocator was already initialized");
        };

        /// The exclusive max address is given by multiplying the number of bytes
        /// in a KB by mem_upper, and adding this to UPPER_MEMORY_START.
        let frames_ceil_address = UPPER_MEMORY_START.saturating_add(mem_upper * KB);

        /// TODO: Do we still need to add the BOOTSTRAP_ALLOCATOR_SIZE
        let frames_base_address = trampoline_heap_top() + BOOTSTRAP_ALLOCATOR_SIZE;

        /// Check to see if dummy_allocator initialized properly (both start and end should be zero)
        let start = dummy_allocator.get_start_address();
        let end = dummy_allocator.get_end_address();
        assert_eq!(start, 0);
        assert_eq!(end, 0);

        /// Set the proper start and end addresses
        dummy_allocator.set_start_address(frames_base_address);
        dummy_allocator.set_end_address(frames_ceil_address);

        let num_frames_in_system = (frames_ceil_address - frames_base_address) /
            (size_of::<CoreMapEntry>() + PAGE_FRAME_SIZE);

        /// This should ALWAYS be the first global allocation to take place - should use dummy allocator
        ///
        println!("[KERNEL ALLOCATOR]: Creating Coremap Entries for Frame Allocator");
        let mut core_map: Box<[CoreMapEntry]> = vec![CoreMapEntry::DEFAULT; num_frames_in_system]
                                                .into_boxed_slice();
        println!("[KERNEL ALLOCATOR]: Finished creating Coremap Entries for Frame Allocator");

        /// Check that the dummy allocator actually updated its internal state
        /// I.e. the start address should have moved to accommodate Coremap Entries
        /// The Coremap should take up 128 frames
        ///
        assert_ne!(frames_base_address, dummy_allocator.get_start_address());
        println!("[KERNEL ALLOCATOR]: Frame Base Address: {}, Dummy Allocator Start Address: {}",
                 frames_base_address, dummy_allocator.get_start_address());

        let mut frame_allocator = FrameAllocatorWrapper::new_in(
            NonNull::new(dummy_allocator.get_start_address() as *mut u8).expect("frames_base can't be null"),
            core_map,
            num_frames_in_system);

        *self.state.get_mut() = KernelAllocatorState::Initialized {
            subblock_allocator: SubblockAllocator::new(
                dummy_allocator.get_start_address() as *mut u8,
                frame_allocator,
            )
        };
    }

    pub unsafe fn frame_alloc(&mut self, frames: usize) -> Result<NonNull<[u8]>, AllocError> {
        let KernelAllocatorState::Initialized {
            subblock_allocator, ..
        } = &mut *self.state.get()
        else {
            return Err(AllocError);
        };

        subblock_allocator.get_frame_allocator().alloc(frames)
    }

    pub unsafe fn frame_dealloc(&mut self, ptr: NonNull<u8>) {
        let KernelAllocatorState::Initialized {
            subblock_allocator, ..
        } = &mut *self.state.get()
        else {
            halt!("[KERNEL ALLOCATOR]: Dealloc called on DeInitialized or SetupState kernel");
        };

        subblock_allocator.get_frame_allocator().dealloc(ptr);
    }


    pub unsafe fn deinit(&mut self) {
        let KernelAllocatorState::Initialized {
            subblock_allocator,
        } = self.state.get_mut()
        else {
            panic!("[KERNEL ALLOCATOR]: deinit called before initialization of kernel allocator");
        };

        let mut incorrect_num_allocs = false;

        if TOTAL_NUM_ALLOCATIONS.load(Ordering::Relaxed) != TOTAL_NUM_DEALLOCATIONS.load(Ordering::Relaxed) {
            incorrect_num_allocs = true;
        }

        subblock_allocator.deinit();

        if incorrect_num_allocs{
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
        if TOTAL_NUM_ALLOCATIONS.load(Ordering::Relaxed) == 0 {
            // If we are here, it should be the dummy allocator doing the allocation
            println!("[KERNEL ALLOCATOR]: Beginning Dummy Allocation for Coremap Entries");

            let KernelAllocatorState::SetupState {
                dummy_allocator
            } = &mut *self.state.get() else {
                halt!("[KERNEL ALLOCATOR]: Kernel initialized before Coremap Entries created")
            };

            let size = layout.size();
            let align = layout.align();

            // The alignment of the layout should never be larger than the size of a page
            if align > MAX_SUPPORTED_ALIGN{
                return ptr::null_mut();
            }

            let num_frames_requested = ((size + align).next_multiple_of(PAGE_FRAME_SIZE))
                                                        / PAGE_FRAME_SIZE;

            let Ok(region) = dummy_allocator.alloc(num_frames_requested) else {
                halt!("[KERNEL ALLOCATOR]: Unable to allocate memory according to provided layout in DummyAllocator");
            };

            // At this point, we know the allocation was successful; increment global statistics
            let new_total_allocs = TOTAL_NUM_ALLOCATIONS.load(Ordering::Relaxed) + 1;
            TOTAL_NUM_ALLOCATIONS.store(new_total_allocs, Ordering::Relaxed);

            region.as_ptr().cast::<u8>()
        } else {
            let KernelAllocatorState::Initialized {
                subblock_allocator,
            } = &mut *self.state.get()
                else {
                    halt!("[KERNEL ALLOCATOR]: Allocation requested before kernel is Initialized");
                };

            let size = layout.size();
            let align = layout.align();

            // The alignment of the layout should never be larger than the size of a page
            if align > MAX_SUPPORTED_ALIGN{
                return ptr::null_mut();
            }

            let num_frames_requested = ((size + align).next_multiple_of(PAGE_FRAME_SIZE))
                / PAGE_FRAME_SIZE;

            /// Allocate using subblock allocator
            let ret_ptr = match subblock_allocator.allocate(layout) {
                Ok(T) => T,
                Err(_) => halt!("[KERNEL ALLOCATOR]: Unable to allocate memory according to provided layout in SubblockAllocator"),
            };

            // At this point, we know the allocation was successful; increment global statistics
            let new_total_allocs = TOTAL_NUM_ALLOCATIONS.load(Ordering::Relaxed) + 1;
            TOTAL_NUM_ALLOCATIONS.store(new_total_allocs, Ordering::Relaxed);

            ret_ptr
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let KernelAllocatorState::Initialized {
            subblock_allocator,
        } = &mut *self.state.get()
        else {
            halt!("[KERNEL ALLOCATOR]: dealloc called before initialization of kernel allocator");
        };

        subblock_allocator.deallocate(ptr, layout);

        let new_total_deallocs = TOTAL_NUM_DEALLOCATIONS.load(Ordering::Relaxed) + 1;
        TOTAL_NUM_DEALLOCATIONS.store(new_total_deallocs, Ordering::Relaxed);
    }
}

fn test_1(){
    let heap_val_1 = Box::new(10);
    let heap_val_2 = Box::new(3.2);
    assert_eq!(*heap_val_1, 10);
    assert_eq!(*heap_val_2, 3.2);
}

fn test_2(){
    let n = 70;
    let mut test_vec = Vec::new();
    for i in 1..=n {
        test_vec.push(i)
    }

    assert_eq!(test_vec[10], 11);
    assert_eq!(test_vec[67], 68);
    assert_eq!(test_vec.iter().sum::<u64>(), (n + 1) * (n / 2));
}

fn test_3(){
    let large_n = 10000;
    let mut large_test_vec = Vec::new();
    for i in 1..=large_n{
        large_test_vec.push(i)
    }

    assert_eq!(large_test_vec[10], 11);
    assert_eq!(large_test_vec[67], 68);
    assert_eq!(large_test_vec.iter().sum::<u64>(), (large_n + 1) * (large_n / 2));
}

// Run tests to see if GlobalAllocator is working properly
#[allow(dead_code)]
pub fn run_allocation_tests(){
    println!("[ALLOCATOR TEST]: Beginning to run allocation tests");

    // Test 1
    println!("[ALLOCATOR TEST]: Running Test 1");
    test_1();
    println!("[ALLOCATOR TEST]: Successfully completed Test 1");

    // Test 2
    println!("[ALLOCATOR TEST]: Running Test 2");
    test_2();
    println!("[ALLOCATOR TEST]: Successfully completed Test 2");

    // Test 3
    // This test will fail because it causes an allocation of request larger than one frame
    println!("[ALLOCATOR TEST]: Running Test 3");
    test_3();
    println!("[ALLOCATOR TEST]: Successfully completed Test 3");

}




