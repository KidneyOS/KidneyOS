use core::alloc::{Layout, GlobalAlloc};

use crate::constants::MB;
use crate::mem::KERNEL_ALLOCATOR;
use crate::println;

type TID = u16;
type ThreadFunction = fn() -> ();

// Current value marks the next avaliable TID value to use.
static mut NEXT_UNRESERVED_TID: TID = 0;
const MAX_ALLOWED_THREADS: u32 = 64; // TEMP.

const THREAD_STACK_SIZE: usize = MB;

enum ThreadStatus {
    Invalid,
    Running,
    Ready,
    Blocked,
    Dying
}

pub struct ThreadControlBlock {

    tid: TID,
    status: ThreadStatus,
    stack_pointer: *mut u8,

}


// NOT PERMANENT.
mod scheduling {
    use alloc::collections::VecDeque;
    use super::ThreadControlBlock;
    use super::TID;

    trait Scheduler {

        fn new() -> Self
        where
            Self: Sized;

        fn push(&mut self, thread: ThreadControlBlock) -> ();
        fn pop(&mut self) -> Option<ThreadControlBlock>;
        fn remove(&mut self, tid: TID) -> bool;

    }

    struct FIFOScheduler {

        ready_queue: VecDeque<ThreadControlBlock>

    }

    impl Scheduler for FIFOScheduler {

        fn new() -> FIFOScheduler {
            return FIFOScheduler {
                ready_queue: VecDeque::new()
            };
        }

        fn push(&mut self, thread: ThreadControlBlock) -> () {

            self.ready_queue.push_back(thread);

        }

        fn pop(&mut self) -> Option<ThreadControlBlock> {

            return self.ready_queue.pop_front();

        }

        fn remove(&mut self, tid: TID) -> bool {
            return false;
        }

    }

}
// NOT PERMANENT.

/**
 * To be called before any other thread functions.
 * To be called with interrupts disabled.
 */
pub fn thread_system_initialization() -> () {

    println!("Initializing Thread Sub-System...");

    println!("Finished Thread initialization.");

}

fn allocate_tid() -> TID {

    unsafe {
        let new_tid = NEXT_UNRESERVED_TID;

        // TODO: Lock.
        NEXT_UNRESERVED_TID += 1;

        return new_tid;
    }

}

// Creates a new TCB to begin executing the specficied function.
// The new thread will be enqueued into the active scheduler.
// Will return the TID allocated to this thread if successful.
fn thread_create(entry_function: ThreadFunction) -> Option<TID> {

    let tid = allocate_tid();

    // Allocate a stack for this thread.
    // In x86 stacks from downward, so we must pass in the top of this memory to the thread.
    let stack_pointer_bottom: *mut u8;
    let stack_pointer_top: *mut u8;
    let layout = Layout::from_size_align(THREAD_STACK_SIZE, 8).unwrap();
    unsafe {
        stack_pointer_bottom = KERNEL_ALLOCATOR.alloc_zeroed(layout);
        stack_pointer_top = stack_pointer_bottom.offset(THREAD_STACK_SIZE.try_into().unwrap());
    }

    // Create our new TCB.
    let mut new_thread = ThreadControlBlock {
        tid,
        status: ThreadStatus::Invalid,
        stack_pointer: stack_pointer_top
    };

    // Place the function to execute at the top of the stack.
    // TODO: Other frames for switching code? Minimum, thread_thunk.

    // Hand this off the the scheduler.
    new_thread.status = ThreadStatus::Ready;

    return Some(tid);

}
