use super::thread_functions::{PrepareThreadContext, SwitchThreadsContext};
use crate::system::{running_thread_ppid, unwrap_system};
use crate::threading::process::{Pid, ProcessState, Tid};
use crate::user_program::elf::{ElfArchitecture, ElfProgramType, ElfUsage};
use crate::{
    fs::fs_manager::FileSystemID,
    paging::{PageManager, PageManagerDefault},
    user_program::elf::Elf,
    vfs::{INodeNum, OwnedPath},
    KERNEL_ALLOCATOR,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::{
    mem::size_of,
    ptr::{copy_nonoverlapping, write_bytes, NonNull},
};
use kidneyos_shared::mem::{OFFSET, PAGE_FRAME_SIZE};

// The stack size choice is based on that of x86-64 Linux and 32-bit Windows
// Linux: https://docs.kernel.org/next/x86/kernel-stacks.html
// Windows: https://techcommunity.microsoft.com/t5/windows-blog-archive/pushing-the-limits-of-windows-processes-and-threads/ba-p/723824
pub const KERNEL_THREAD_STACK_FRAMES: usize = 2;
const KERNEL_THREAD_STACK_SIZE: usize = KERNEL_THREAD_STACK_FRAMES * PAGE_FRAME_SIZE;
pub const USER_THREAD_STACK_FRAMES: usize = 4 * 1024;
pub const USER_THREAD_STACK_SIZE: usize = USER_THREAD_STACK_FRAMES * PAGE_FRAME_SIZE;
pub const USER_STACK_BOTTOM_VIRT: usize = 0x100000;

#[allow(unused)]
#[derive(PartialEq, Debug)]
pub enum ThreadStatus {
    Invalid,
    Running,
    Ready,
    Blocked,
    Dying,
}

pub struct ProcessControlBlock {
    pub pid: Pid,
    // The Pid of the process' parent
    pub ppid: Pid,
    // The TIDs of this process' children threads
    pub child_tids: Vec<Tid>,
    // The TIDs of the threads waiting on this process to end
    pub waiting_thread: Option<Tid>,

    pub exit_code: Option<i32>,
    /// filesystem and inode of current working directory
    pub cwd: (FileSystemID, INodeNum),
    /// path to cwd (needed for getcwd syscall)
    pub cwd_path: OwnedPath,
}

impl ProcessControlBlock {
    pub fn create(state: &mut ProcessState, parent_pid: Pid) -> Pid {
        let pid = state.allocate_pid();
        let mut root = crate::fs::fs_manager::ROOT.lock();
        // open stdin, stdout, stderr
        root.open_standard_fds(pid);
        let pcb = Self {
            pid,
            ppid: parent_pid,
            child_tids: Vec::new(),
            waiting_thread: None,
            exit_code: None,
            cwd: root.get_root().unwrap(),
            cwd_path: "/".into(),
        };

        state.table.add(Box::new(pcb));

        pid
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
    // If true, we'll make an effort to run this thread in kernel mode.
    // Otherwise, we'll run this thread in user mode.
    pub is_kernel: bool,
    pub status: ThreadStatus,
    pub exit_code: Option<i32>,
    pub page_manager: PageManager,
}

impl ThreadControlBlock {
    pub fn new_from_elf(elf: Elf, state: &mut ProcessState) -> ThreadControlBlock {
        // Shared ELFs can count as a "Relocatable Executable" if the entry point is set.
        let executable = matches!(elf.header.usage, ElfUsage::Executable | ElfUsage::Shared);

        if elf.header.architecture != ElfArchitecture::X86 && executable {
            panic!("ELF was valid, but it was not an executable or it did not target the host platform (x86)");
        }

        let ppid = unsafe {
            unwrap_system()
                .threads
                .running_thread
                .as_ref()
                .map_or(0, |_| running_thread_ppid())
        };

        let pid: Pid = ProcessControlBlock::create(state, ppid);

        let mut page_manager = PageManager::default();

        for program_header in elf.program_headers {
            if program_header.program_type != ElfProgramType::Load {
                continue;
            }

            // Some ELF files have off-alignment segments (off 4KB).
            // We need to pad this space with zeroes.
            let segment_virtual_frame_start =
                program_header.virtual_address as usize / PAGE_FRAME_SIZE;
            let segment_virtual_start = segment_virtual_frame_start * PAGE_FRAME_SIZE;
            let segment_padding = program_header.virtual_address as usize % PAGE_FRAME_SIZE;
            let segment_padded_size = segment_padding + program_header.data.len();

            let frames = segment_padded_size.div_ceil(PAGE_FRAME_SIZE);

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
                    segment_virtual_start,
                    frames * PAGE_FRAME_SIZE,
                    program_header.writable,
                    true,
                );

                write_bytes(kernel_virt_addr, 0, segment_padded_size);

                // Load so we can write to the virtual addresses mapped above.
                copy_nonoverlapping(
                    program_header.data.as_ptr(),
                    kernel_virt_addr.add(segment_padding),
                    program_header.data.len(),
                );

                // Zero the sliver of addresses between the end of the region, and
                // the end of the region we had to map due to page
                write_bytes(
                    kernel_virt_addr.add(segment_padded_size),
                    0,
                    frames * PAGE_FRAME_SIZE - segment_padded_size,
                );
            }
        }

        ThreadControlBlock::new_with_page_manager(
            NonNull::new(elf.header.program_entry as *mut u8)
                .expect("fail to create PCB entry point"),
            pid,
            page_manager,
            state,
        )
    }

    pub fn new_with_page_manager(
        entry_instruction: NonNull<u8>,
        pid: Pid,
        page_manager: PageManager,
        state: &mut ProcessState,
    ) -> Self {
        let mut new_thread = Self::new(entry_instruction, false, pid, page_manager, state);

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

    #[allow(unused)]
    pub fn new_with_setup(eip: NonNull<u8>, is_kernel: bool, state: &mut ProcessState) -> Self {
        let mut new_thread = Self::new(eip, is_kernel, state.allocate_pid(), PageManager::default(), state);

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
                .cast::<PrepareThreadContext>() = PrepareThreadContext::new(eip.as_ptr());
            *switch_threads_context
                .as_ptr()
                .cast::<SwitchThreadsContext>() = SwitchThreadsContext::new();
        }

        new_thread.eip = eip; // !!!
        
        // Our thread can now be run via the `switch_threads` method.
        new_thread.status = ThreadStatus::Ready;
        new_thread
    }

    pub fn new(
        entry_instruction: NonNull<u8>,
        is_kernel: bool,
        pid: Pid,
        mut page_manager: PageManager,
        state: &mut ProcessState,
    ) -> Self {
        let tid: Tid = state.allocate_tid();

        let (kernel_stack, kernel_stack_pointer, user_stack) = Self::map_stacks(&mut page_manager);

        // Create our new TCB.
        Self {
            kernel_stack_pointer,
            kernel_stack,
            eip: NonNull::new(entry_instruction.as_ptr()).expect("failed to create eip"),
            esp: NonNull::new((USER_STACK_BOTTOM_VIRT + USER_THREAD_STACK_SIZE) as *mut u8)
                .expect("failed to create esp"),
            user_stack,
            tid,
            pid, // Potentially could be swapped to directly copy the pid of the running thread
            is_kernel,
            status: ThreadStatus::Invalid,
            exit_code: None,
            page_manager,
        }
    }

    fn map_stacks(page_manager: &mut PageManager) -> (NonNull<u8>, NonNull<u8>, NonNull<u8>) {
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
        (kernel_stack, kernel_stack_pointer_top, user_stack)
    }

    /// Creates the 'kernel thread'.
    ///
    /// # Safety
    /// Should only be used once while starting the threading system.
    pub fn new_kernel_thread(page_manager: PageManager, state: &mut ProcessState) -> Self {
        ThreadControlBlock {
            kernel_stack_pointer: NonNull::dangling(), // This will be set in the context switch immediately following.
            kernel_stack: NonNull::dangling(),
            eip: NonNull::dangling(),
            esp: NonNull::dangling(),
            user_stack: NonNull::dangling(),
            tid: state.allocate_tid(),
            pid: state.allocate_pid(),
            is_kernel: true,
            status: ThreadStatus::Running,
            exit_code: None,
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

    pub fn set_exit_code(&mut self, exit_code: i32) {
        self.exit_code = Some(exit_code);
    }

    pub fn reap(&mut self) {
        assert_eq!(
            self.status,
            ThreadStatus::Dying,
            "A thread must be dying to be reaped."
        );

        // Most of the TCB is dropped automatically.
        // But the stack must be manually deallocated.
        // However, the first TCB is the kernel stack and not treated as such.
        if self.tid != 0 {
            self.kernel_stack_pointer = NonNull::dangling();

            self.eip = NonNull::dangling();
            self.esp = NonNull::dangling();

            // TODO: drop up alloc'd memory
        }

        self.status = ThreadStatus::Invalid;
    }
}
