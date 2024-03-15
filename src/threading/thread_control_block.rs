use core::alloc::{Allocator, Layout};
use core::mem::size_of;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU16, Ordering};

use alloc::alloc::Global;

use crate::constants::KB;
use crate::threading::thread_functions::{
    PrepareThreadContext, RunThreadContext, SwitchThreadsContext, ThreadFunction,
};

pub type Tid = u16;

// Current value marks the next avaliable TID value to use.
static mut NEXT_UNRESERVED_TID: AtomicU16 = AtomicU16::new(0);

pub const THREAD_STACK_SIZE: usize = KB * 4;

pub enum ThreadStatus {
    Invalid,
    Running,
    Ready,
    Blocked,
    Dying,
}

#[repr(C, packed)]
pub struct ThreadControlBlock {
    pub stack_pointer: NonNull<u8>, // Must be kept as the top of the struct so it has the same address as the TCB.
    stack_pointer_bottom: NonNull<u8>, // Kept to avoid dropping the stack and to detect overflows.

    pub tid: Tid,
    pub status: ThreadStatus,

    pub context: SwitchThreadsContext, // Not always valid. TODO: Use type system here, worried about use in inline assembly and ownership.
}

pub fn allocate_tid() -> Tid {
    // SAFETY: Atomically accesses a shared variable.
    unsafe { NEXT_UNRESERVED_TID.fetch_add(1, Ordering::SeqCst) as Tid }
}

impl ThreadControlBlock {
    pub fn create(entry_function: ThreadFunction) -> Self {
        let tid: Tid = allocate_tid();

        // Allocate a stack for this thread.
        // In x86 stacks from downward, so we must pass in the top of this memory to the thread.
        let stack_pointer_bottom;
        let stack_pointer_top;
        let layout =
            Layout::from_size_align(THREAD_STACK_SIZE, 8).expect("Could not create layout.");

        // SAFETY: Using raw Nonnull pointers.
        unsafe {
            stack_pointer_bottom = Global
                .allocate_zeroed(layout)
                .expect("Could not allocate stack.");
            stack_pointer_top = NonNull::new(
                stack_pointer_bottom
                    .as_ptr()
                    .cast::<u8>()
                    .add(THREAD_STACK_SIZE),
            )
            .expect("Could not determine end of stack.");
        }

        // Create our new TCB.
        let mut new_thread = Self {
            tid,
            status: ThreadStatus::Invalid,
            stack_pointer: stack_pointer_top,
            stack_pointer_bottom: NonNull::new(stack_pointer_bottom.as_ptr().cast::<u8>())
                .expect("Error converting stack."),
            context: SwitchThreadsContext::empty_context(),
        };

        // Now, we must build the stack frames for our new thread.
        // In order (of creation), we have:
        //  * run_thread frame
        //  * prepare_thread frame
        //  * switch_threads frame

        let run_thread_context = new_thread
            .allocate_stack_space(size_of::<RunThreadContext>())
            .expect("No Stack Space!");
        let prepare_thread_context = new_thread
            .allocate_stack_space(size_of::<PrepareThreadContext>())
            .expect("No Stack Space!");
        let switch_threads_context = new_thread
            .allocate_stack_space(size_of::<SwitchThreadsContext>())
            .expect("No Stack Space!");

        // SAFETY: Manually setting stack bytes ala C.
        unsafe {
            *run_thread_context.as_ptr().cast::<RunThreadContext>() =
                RunThreadContext::create(entry_function);
            *prepare_thread_context
                .as_ptr()
                .cast::<PrepareThreadContext>() = PrepareThreadContext::create();
            *switch_threads_context
                .as_ptr()
                .cast::<SwitchThreadsContext>() = SwitchThreadsContext::create();
        }

        // Our thread can now be run via the `switch_threads` method.
        new_thread.status = ThreadStatus::Ready;
        new_thread
    }

    /**
     * If possible without stack-smashing, moves the stack pointer down and returns the new value.
     */
    pub fn allocate_stack_space(&mut self, bytes: usize) -> Option<NonNull<u8>> {
        if !self.has_stack_space(bytes) {
            return None;
        }

        Some(self.shift_stack_pointer_down(bytes))
    }

    /**
     * Check if `bytes` bytes will fit on the stack.
     */
    pub const fn has_stack_space(&self, bytes: usize) -> bool {
        // SAFETY: Calculates the distance between the top and bottom of the stack pointers.
        let avaliable_space =
            unsafe { self.stack_pointer.offset_from(self.stack_pointer_bottom) as usize };

        avaliable_space >= bytes
    }

    /**
     * Moves the stack pointer down and returns the new position.
     */
    pub fn shift_stack_pointer_down(&mut self, amount: usize) -> NonNull<u8> {
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
