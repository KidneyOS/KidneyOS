use super::thread_functions::{PrepareThreadContext, SwitchThreadsContext, ThreadFunction};
use crate::{
    paging::{PageManager, PageManagerDefault},
    user_program::{
        elf_loader::parse_elf,
        virtual_memory_area::{VmAreaStruct, VmFlags},
    },
    KERNEL_ALLOCATOR,
};
use alloc::vec::Vec;
use core::{
    mem::size_of,
    ptr::{copy_nonoverlapping, write_bytes, NonNull},
    sync::atomic::{AtomicU16, Ordering},
};
use kidneyos_shared::mem::{OFFSET, PAGE_FRAME_SIZE};

pub type Pid = u16;
pub type Tid = u16;

// Current value marks the next available PID & TID values to use.
static NEXT_UNRESERVED_PID: AtomicU16 = AtomicU16::new(0);
static NEXT_UNRESERVED_TID: AtomicU16 = AtomicU16::new(0);

// The stack size choice is based on that of x86-64 Linux and 32-bit Windows
// Linux: https://docs.kernel.org/next/x86/kernel-stacks.html
// Windows: https://techcommunity.microsoft.com/t5/windows-blog-archive/pushing-the-limits-of-windows-processes-and-threads/ba-p/723824
pub const KERNEL_THREAD_STACK_FRAMES: usize = 2;
const KERNEL_THREAD_STACK_SIZE: usize = KERNEL_THREAD_STACK_FRAMES * PAGE_FRAME_SIZE;
pub const USER_THREAD_STACK_FRAMES: usize = 4 * 1024;
pub const USER_THREAD_STACK_SIZE: usize = USER_THREAD_STACK_FRAMES * PAGE_FRAME_SIZE;
pub const USER_STACK_BOTTOM_VIRT: usize = 0x100000;

#[allow(unused)]
#[derive(PartialEq)]
pub enum ThreadStatus {
    Invalid,
    Running,
    Ready,
    Blocked,
    Dying,
}

pub fn allocate_pid() -> Pid {
    // SAFETY: Atomically accesses a shared variable.
    NEXT_UNRESERVED_PID.fetch_add(1, Ordering::SeqCst) as Pid
}
pub fn allocate_tid() -> Tid {
    // SAFETY: Atomically accesses a shared variable.
    NEXT_UNRESERVED_TID.fetch_add(1, Ordering::SeqCst) as Tid
}

pub struct ProcessControlBlock {
    pub pid: Pid,
    // The TIDs of this process' children threads
    pub child_tids: Vec<Tid>,
    // The TIDs of the threads waiting on this process to end
    pub wait_list: Vec<Tid>,

    // TODO: (file I/O) file descriptor table
    pub exit_code: Option<i32>,
}

impl ProcessControlBlock {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(elf_data: &[u8]) -> ThreadControlBlock {
        let pid: Pid = allocate_pid();

        let (entrypoint, vm_areas) =
            parse_elf(elf_data).expect("init process's ELF data was malformed");

        let mut page_manager = PageManager::default();
        for VmAreaStruct {
            vm_start,
            vm_end,
            offset,
            // TODO: Consider all the flags. For those we can support, implement
            // it. For those we can't, throw an error if they're set in such a
            // way that the program might not work correctly.
            flags: VmFlags { write, .. },
        } in vm_areas
        {
            let len = vm_end - vm_start;
            let frames = len.div_ceil(PAGE_FRAME_SIZE);

            unsafe {
                // TODO: Save this physical address somewhere so we can deallocate
                // it when dropping the thread.
                let kernel_virt_addr = KERNEL_ALLOCATOR
                    .frame_alloc(frames)
                    .expect("no more frames...")
                    .cast::<u8>()
                    .as_ptr();
                let phys_addr = kernel_virt_addr.sub(OFFSET);

                // TODO: Throw an error if this range overlaps any previously mapped
                // ranges, since `map_range` requires that the input range has not
                // already been mapped.

                // Map the physical address obtained by the allocation above to the
                // virtual address assigned by the ELF header.
                page_manager.map_range(
                    phys_addr as usize,
                    vm_start,
                    frames * PAGE_FRAME_SIZE,
                    write,
                    true,
                );

                // Load so we can write to the virtual addresses mapped above.
                copy_nonoverlapping(&elf_data[offset] as *const u8, kernel_virt_addr, len);

                // Zero the sliver of addresses between the end of the region, and
                // the end of the region we had to map due to page
                write_bytes(kernel_virt_addr.add(len), 0, frames * PAGE_FRAME_SIZE - len);
            }
        }

        let mut new_pcb = Self {
            pid,
            child_tids: Vec::new(),
            wait_list: Vec::new(),
            exit_code: None,
        };
        let new_tcb = ThreadControlBlock::new_with_elf(
            NonNull::new(entrypoint as *mut u8).expect("fail to create PCB entry point"),
            pid,
            page_manager,
        );
        new_pcb.child_tids.push(new_tcb.tid);

        new_tcb
    }
}

// TODO: Use enums so that we never have garbage data (i.e. stacks that don't
// need be freed for the kernel thread, information that doesn't make sense when
// the thread is in certain states, etc.)
pub struct ThreadControlBlock {
    pub kernel_stack_pointer: NonNull<u8>,
    // Kept so we can free the kernel stack later.
    pub kernel_stack: NonNull<u8>,

    // The user virtual address containing the user instruction pointer to
    // switch to next time this thread is run.
    pub eip: NonNull<u8>,
    // Like above, but the stack pointer.
    pub esp: NonNull<u8>,
    // The kernel virtual address of the user stack, so it can be freed later.
    pub user_stack: NonNull<u8>,

    pub tid: Tid,
    // The PID of the parent PCB.
    pub pid: Pid,
    pub status: ThreadStatus,
    pub page_manager: PageManager,
}

impl ThreadControlBlock {
    pub fn new(entry_function: ThreadFunction, pid: Pid) -> Self {
        // let eip = unsafe { transmute(entry_function) };
        let eip = NonNull::new(entry_function as *mut u8).expect("Null entry function given!");
        let mut new_thread = Self::new_(eip, pid, PageManager::default());

        // Now, we must build the stack frames for our new thread.
        // In order (of creation), we have:
        //  * prepare_thread frame
        //  * switch_threads
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

        new_thread.eip = prepare_thread_context;

        // Our thread can now be run via the `switch_threads` method.
        new_thread.status = ThreadStatus::Ready;
        new_thread
    }

    pub fn new_with_elf(
        entry_instruction: NonNull<u8>,
        pid: Pid,
        page_manager: PageManager,
    ) -> Self {
        let mut new_thread = Self::new_(entry_instruction, pid, page_manager);

        // Now, we must build the stack frames for our new thread.
        let switch_threads_context = new_thread
            .allocate_stack_space(size_of::<SwitchThreadsContext>())
            .expect("No Stack Space!");

        // SAFETY: Manually setting stack bytes a la C.
        unsafe {
            *switch_threads_context
                .as_ptr()
                .cast::<SwitchThreadsContext>() = SwitchThreadsContext::new();
        }

        // Our thread can now be run via the `switch_threads` method.
        new_thread.status = ThreadStatus::Ready;
        new_thread
    }

    fn new_(entry_instruction: NonNull<u8>, pid: Pid, mut page_manager: PageManager) -> Self {
        let tid: Tid = allocate_tid();

        // Allocate a kernel stack for this thread. In x86 stacks grow downward,
        // so we must pass in the top of this memory to the thread.
        let (kernel_stack, kernel_stack_pointer_top);
        unsafe {
            kernel_stack = KERNEL_ALLOCATOR
                .frame_alloc(KERNEL_THREAD_STACK_FRAMES)
                .expect("could not allocate kernel stack")
                .cast::<u8>();
            kernel_stack_pointer_top = kernel_stack.add(KERNEL_THREAD_STACK_SIZE);
            write_bytes(kernel_stack.as_ptr(), 0, KERNEL_THREAD_STACK_SIZE);
        }

        // TODO: We should only do this if there wasn't already a stack section
        // defined in the ELF file.
        let user_stack;
        unsafe {
            user_stack = KERNEL_ALLOCATOR
                .frame_alloc(USER_THREAD_STACK_FRAMES)
                .expect("could not allocate user stack")
                .cast::<u8>();
            page_manager.map_range(
                user_stack.as_ptr() as usize - OFFSET,
                // TODO: This shouldn't be hardcoded, we need to ensure the ELF
                // didn't already declare a stack section (we should be using
                // that if it did), and that this doesn't overlap with any
                // existing regions.
                USER_STACK_BOTTOM_VIRT,
                USER_THREAD_STACK_SIZE,
                true,
                true,
            );
        }

        // Create our new TCB.
        Self {
            kernel_stack_pointer: kernel_stack_pointer_top,
            kernel_stack,
            eip: NonNull::new(entry_instruction.as_ptr()).expect("failed to create eip"),
            esp: NonNull::new((USER_STACK_BOTTOM_VIRT + USER_THREAD_STACK_SIZE) as *mut u8)
                .expect("failed to create esp"),
            user_stack,
            tid,
            pid, // Potentially could be swapped to directly copy the pid of the running thread
            status: ThreadStatus::Invalid,
            page_manager,
        }
    }

    /// Creates the 'kernel thread'.
    ///
    /// # Safety
    /// Should only be used once while starting the threading system.
    pub unsafe fn new_kernel_thread(page_manager: PageManager) -> Self {
        ThreadControlBlock {
            kernel_stack_pointer: NonNull::dangling(), // This will be set in the context switch immediately following.
            kernel_stack: NonNull::dangling(),
            eip: NonNull::dangling(),
            esp: NonNull::dangling(),
            user_stack: NonNull::dangling(),
            tid: allocate_tid(),
            pid: allocate_pid(),
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

    /// Check if `bytes` bytes will fit on the kernel stack.
    const fn has_stack_space(&self, bytes: usize) -> bool {
        // SAFETY: Calculates the distance between the top and bottom of the kernel stack pointers.
        let available_space =
            unsafe { self.kernel_stack_pointer.offset_from(self.kernel_stack) as usize };

        available_space >= bytes
    }

    /// Moves the stack pointer down and returns the new position.
    fn shift_stack_pointer_down(&mut self, amount: usize) -> NonNull<u8> {
        // SAFETY: `has_stack_space` must have returned true for this amount before calling.
        unsafe {
            let raw_pointer = self.kernel_stack_pointer.as_ptr().cast::<u8>();
            let new_pointer =
                NonNull::new(raw_pointer.sub(amount)).expect("Error shifting stack pointer.");
            self.kernel_stack_pointer = new_pointer;
            self.kernel_stack_pointer
        }
    }
}
