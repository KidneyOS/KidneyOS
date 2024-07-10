use super::thread_functions::SwitchThreadsContext;
use crate::{
    paging::{PageManager, PageManagerDefault},
    user_program::{
        elf_loader::parse_elf,
        virtual_memory_area::{VmAreaStruct, VmFlags},
    },
    KERNEL_ALLOCATOR,
};
use core::{
    mem::size_of,
    ptr::{copy_nonoverlapping, write_bytes, NonNull},
    // sync::atomic::{AtomicU16, Ordering},
};
use kidneyos_shared::mem::{OFFSET, PAGE_FRAME_SIZE};

pub type Tid = u16;

// Current value marks the next avaliable TID value to use.
// static NEXT_UNRESERVED_TID: AtomicU16 = AtomicU16::new(0);

// The stack size choice is based on that of x86-64 Linux and 32-bit Windows
// Linux: https://docs.kernel.org/next/x86/kernel-stacks.html
// Windows: https://techcommunity.microsoft.com/t5/windows-blog-archive/pushing-the-limits-of-windows-processes-and-threads/ba-p/723824
pub const KERNEL_THREAD_STACK_FRAMES: usize = 2;
const KERNEL_THREAD_STACK_SIZE: usize = KERNEL_THREAD_STACK_FRAMES * PAGE_FRAME_SIZE;
pub const USER_THREAD_STACK_FRAMES: usize = 4 * 1024;
pub const USER_THREAD_STACK_SIZE: usize = USER_THREAD_STACK_FRAMES * PAGE_FRAME_SIZE;
pub const USER_STACK_BOTTOM_VIRT: usize = 0x100000;

#[allow(unused)]
#[derive(PartialEq, Clone)]
pub enum ThreadStatus {
    Invalid,
    Running,
    Ready,
    Blocked,
    Dying,
}

// TODO: Use enums so that we never have garbage data (i.e. stacks that don't
// need be freed for the kernel thread, information that doesn't make sense when
// the thread is in certain states, etc.)
#[derive(Clone)]
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
    pub status: ThreadStatus,
    pub page_manager: PageManager,
}

// pub fn allocate_tid() -> Tid {
//     NEXT_UNRESERVED_TID.fetch_add(1, Ordering::SeqCst) as Tid
// }

impl ThreadControlBlock {
    pub fn create(elf_data: &[u8]) -> Self {

        // let tid: Tid = unsafe {
        //     THREAD_MANAGER
        //         .as_mut()
        //         .expect("No Thread Manager set up!")
        //         .allocate_tid()
        // };

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
        let mut new_thread = Self {
            tid: 0,  // always invalid until added to thread manager.
            status: ThreadStatus::Invalid,
            kernel_stack_pointer: kernel_stack_pointer_top,
            kernel_stack,
            eip: NonNull::new(entrypoint as *mut u8).expect("failed to create eip"),
            esp: NonNull::new((USER_STACK_BOTTOM_VIRT + USER_THREAD_STACK_SIZE) as *mut u8)
                .expect("failed to create esp"),
            user_stack,
            page_manager,
        };

        // Now, we must build the stack frames for our new thread.
        // In order (of creation), we have:
        //  * prepare_thread frame
        //  * switch_threads frame

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

    /// Creates the 'kernel thread'.
    ///
    /// # Safety
    /// Should only be used once while starting the threading system.
    pub unsafe fn create_kernel_thread(page_manager: PageManager) -> Self {
        // let tid: Tid = unsafe {
        //     THREAD_MANAGER
        //         .as_mut()
        //         .expect("No Thread Manager set up!")
        //         .allocate_tid()
        // };

        ThreadControlBlock {
            kernel_stack_pointer: NonNull::dangling(), // This will be set in the context switch immediately following.
            kernel_stack: NonNull::dangling(),
            eip: NonNull::dangling(),
            esp: NonNull::dangling(),
            user_stack: NonNull::dangling(),
            tid: 0,  // always invalid until added to thread manager.
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
            unsafe { self.kernel_stack_pointer.offset_from(self.kernel_stack) as usize };

        avaliable_space >= bytes
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
