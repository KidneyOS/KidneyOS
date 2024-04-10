use super::thread_functions::{PrepareThreadContext, SwitchThreadsContext, ThreadFunction};
use crate::{paging::PageManager, KERNEL_ALLOCATOR};
use core::{
    mem::size_of,
    ptr::{write_bytes, NonNull},
    sync::atomic::{AtomicU16, Ordering},
};
use kidneyos_shared::mem::{OFFSET, PAGE_FRAME_SIZE};

pub type Tid = u16;

// Current value marks the next avaliable TID value to use.
static mut NEXT_UNRESERVED_TID: AtomicU16 = AtomicU16::new(0);

pub const THREAD_STACK_FRAMES: usize = 4 * 1024;
pub const STACK_BOTTOM_VADDR: usize = 0x100000; // TODO: How should this be determined? We need to make sure it doesn't overlap with anything else in the ELF file. We also need to make sure the elf file doesn't have its own stack segment defined.

#[allow(unused)]
#[derive(PartialEq)]
pub enum ThreadStatus {
    Invalid,
    Running,
    Ready,
    Blocked,
    Dying,
}

#[repr(C, packed)]
pub struct ThreadControlBlock {
    // TODO: Change the stack pointer type and remove the need to keep the bottom of the stack.
    stack_pointer: NonNull<u8>, // Must be kept as the top of the struct so it has the same address as the TCB.
    stack_pointer_bottom: NonNull<u8>, // Kept to avoid dropping the stack and to detect overflows.

    // True if this is a kernel thread to be executed in privileged mode.
    kernel: bool,

    pub tid: Tid,
    pub status: ThreadStatus,
    pub page_manager: PageManager,
}

pub fn allocate_tid() -> Tid {
    // SAFETY: Atomically accesses a shared variable.
    unsafe { NEXT_UNRESERVED_TID.fetch_add(1, Ordering::SeqCst) as Tid }
}

impl ThreadControlBlock {
    pub fn create(entry_function: ThreadFunction, mut page_manager: PageManager) -> Self {
        let tid: Tid = allocate_tid();

        // Allocate a stack for this thread.
        // In x86 stacks from downward, so we must pass in the top of this memory to the thread.
        let stack_pointer_bottom = unsafe { KERNEL_ALLOCATOR.frame_alloc(THREAD_STACK_FRAMES) }
            .expect("Could not allocate stack.");

        unsafe {
            page_manager.map_range(
                stack_pointer_bottom.cast::<u8>().as_ptr() as usize - OFFSET,
                STACK_BOTTOM_VADDR,
                THREAD_STACK_FRAMES * PAGE_FRAME_SIZE,
                true,
                true,
            );
            write_bytes(
                stack_pointer_bottom.cast::<u8>().as_ptr(),
                0,
                THREAD_STACK_FRAMES * PAGE_FRAME_SIZE,
            );
        }

        // Create our new TCB.
        let mut new_thread = Self {
            tid,
            status: ThreadStatus::Invalid,
            kernel: false,
            stack_pointer: NonNull::new(
                (STACK_BOTTOM_VADDR + THREAD_STACK_FRAMES * PAGE_FRAME_SIZE) as *mut u8,
            )
            .unwrap(), // TODO: off by 1?
            stack_pointer_bottom: NonNull::new(STACK_BOTTOM_VADDR as *mut u8)
                .expect("Error converting stack."),
            page_manager,
        };

        // Now, we must build the stack frames for our new thread.
        // In order (of creation), we have:
        //  * prepare_thread frame
        //  * switch_threads frame

        let prepare_thread_context = new_thread
            .allocate_stack_space(size_of::<PrepareThreadContext>())
            .expect("No Stack Space!");
        let switch_threads_context = new_thread
            .allocate_stack_space(size_of::<SwitchThreadsContext>())
            .expect("No Stack Space!");

        // SAFETY: Manually setting stack bytes a la C.
        unsafe {
            *prepare_thread_context
                .as_ptr()
                .cast::<PrepareThreadContext>() = PrepareThreadContext::new(entry_function);
            *switch_threads_context
                .as_ptr()
                .cast::<SwitchThreadsContext>() = SwitchThreadsContext::new();
        }

        // Our thread can now be run via the `switch_threads` method.
        new_thread.status = ThreadStatus::Ready;
        new_thread
    }

    /// Creates the 'kernel thread'.
    ///
    /// # Safety
    /// Should only be used once while starting the threading system.
    pub unsafe fn create_kernel_thread(page_manager: PageManager) -> Self {
        ThreadControlBlock {
            stack_pointer: core::ptr::NonNull::dangling(), // This will be set in the context switch immediately following.
            stack_pointer_bottom: core::ptr::NonNull::dangling(), // TODO: Is this ok left dangling? Special case code is required otherwise.
            kernel: true,
            tid: allocate_tid(),
            status: ThreadStatus::Running,
            page_manager,
        }
    }

    /// If possible without stack-smashing, moves the stack pointer down and returns the new value.
    fn allocate_stack_space(&mut self, bytes: usize) -> Option<NonNull<u8>> {
        if !self.has_stack_space(bytes) {
            return None;
        }

        Some(self.shift_stack_pointer_down(bytes))
    }

    /// Check if `bytes` bytes will fit on the stack.
    const fn has_stack_space(&self, bytes: usize) -> bool {
        // SAFETY: Calculates the distance between the top and bottom of the stack pointers.
        let avaliable_space =
            unsafe { self.stack_pointer.offset_from(self.stack_pointer_bottom) as usize };

        avaliable_space >= bytes
    }

    /// Moves the stack pointer down and returns the new position.
    fn shift_stack_pointer_down(&mut self, amount: usize) -> NonNull<u8> {
        // SAFETY: `has_stack_space` must have returned true for this amount before calling.
        unsafe {
            let raw_pointer = self.stack_pointer.as_ptr().cast::<u8>();
            let new_pointer =
                NonNull::new(raw_pointer.sub(amount)).expect("Error shifting stack pointer.");
            self.stack_pointer = new_pointer;
            self.stack_pointer
        }
    }
}
