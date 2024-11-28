use super::thread_functions::{SwitchThreadsContext, ThreadFunction};
use crate::system::{root_filesystem, running_thread_ppid, unwrap_system};
use crate::threading::process::{Pid, ProcessState, Tid};
use crate::user_program::elf::{ElfArchitecture, ElfProgramType, ElfUsage};
use crate::{
    fs::fs_manager::FileSystemID,
    mem::vma::{VMAInfo, VMAList, VMA},
    paging::{PageManager, PageManagerDefault},
    user_program::elf::Elf,
    vfs::{INodeNum, OwnedPath},
    KERNEL_ALLOCATOR,
};
use alloc::vec::Vec;
use core::cmp::max;
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
    pub vmas: VMAList,
}

impl ProcessControlBlock {
    pub fn create(state: &ProcessState, parent_pid: Pid) -> Pid {
        let pid = state.allocate_pid();
        let mut root = root_filesystem().lock();
        // open stdin, stdout, stderr
        root.open_standard_fds(pid);
        // TODO: inherit cwd from parent
        let cwd = root.get_root().unwrap();
        drop(root);
        let mut vmas = VMAList::new();
        // set up stack
        // TODO: Handle stack section defined in the ELF file?
        let stack_avail = vmas.add_vma(
            VMA::new(VMAInfo::Stack, USER_THREAD_STACK_SIZE, true),
            USER_STACK_BOTTOM_VIRT,
        );
        assert!(stack_avail, "stack virtual address range not available");

        let pcb = Self {
            pid,
            ppid: parent_pid,
            child_tids: Vec::new(),
            waiting_thread: None,
            exit_code: None,
            vmas,
            cwd,
            cwd_path: "/".into(),
        };

        state.table.add(pcb);

        pid
    }
}

// TODO: Use enums so that we never have garbage data (i.e. stacks that don't
// need be freed for the kernel thread, information that doesn't make sense when
// the thread is in certain states, etc.)
#[derive(Debug)]
pub struct ThreadControlBlock {
    pub kernel_stack_pointer: NonNull<u8>,
    // Kept so we can free the kernel stack later.
    pub kernel_stack: NonNull<u8>,

    // The user virtual address containing the user instruction pointer to
    // switch to next time this thread is run.
    pub eip: NonNull<u8>,
    // Like above, but the stack pointer.
    pub esp: NonNull<u8>,

    pub tid: Tid,
    // The PID of the parent PCB.
    pub pid: Pid,
    // If true, we'll make an effort to run this thread in kernel mode.
    // Otherwise, we'll run this thread in user mode.
    pub is_kernel: bool,
    // Argument that will be passed to the thread on startup (via stack).
    pub argument: u32,
    pub status: ThreadStatus,
    pub exit_code: Option<i32>,
    pub page_manager: PageManager,
}

#[derive(Debug)]
pub enum ThreadElfCreateError {
    UnsupportedArchitecture,
    NotExecutable,
    InvalidEntryPoint,
}

impl ThreadControlBlock {
    pub fn new_from_elf(
        elf: Elf,
        argument: u32,
        state: &ProcessState,
    ) -> Result<ThreadControlBlock, ThreadElfCreateError> {
        // Shared ELFs can count as a "Relocatable Executable" if the entry point is set.
        let executable = matches!(elf.header.usage, ElfUsage::Executable | ElfUsage::Shared);

        if !executable {
            return Err(ThreadElfCreateError::NotExecutable);
        }

        if elf.header.architecture != ElfArchitecture::X86 {
            return Err(ThreadElfCreateError::UnsupportedArchitecture);
        }

        let any_running_thread = unwrap_system().threads.running_thread.lock().is_some();
        let ppid = if !any_running_thread {
            0
        } else {
            running_thread_ppid()
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
            let segment_size = max(
                program_header.memory_size as usize,
                program_header.data.len(),
            );
            let segment_padded_size = segment_padding + segment_size;

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

        Ok(ThreadControlBlock::new_with_page_manager(
            NonNull::new(elf.header.program_entry as *mut u8)
                .ok_or(ThreadElfCreateError::InvalidEntryPoint)?,
            argument,
            pid,
            page_manager,
            state,
        ))
    }

    pub fn new_with_page_manager(
        entry_instruction: NonNull<u8>,
        argument: u32,
        pid: Pid,
        page_manager: PageManager,
        state: &ProcessState,
    ) -> Self {
        let mut new_thread =
            Self::new(entry_instruction, false, argument, pid, page_manager, state);

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
    pub fn new_with_setup(
        eip: ThreadFunction,
        is_kernel: bool,
        argument: u32,
        state: &mut ProcessState,
    ) -> Self {
        let entry = NonNull::new(eip as *mut u8).unwrap();

        let mut new_thread = Self::new(
            entry,
            is_kernel,
            argument,
            state.allocate_pid(),
            PageManager::default(),
            state,
        );

        // Now, we must build the stack frames for our new thread.
        // In order (of creation), we have:
        //  * switch_threads
        let switch_threads_context = new_thread
            .allocate_stack_space(size_of::<SwitchThreadsContext>())
            .expect("No Stack Space!");

        // SAFETY: Manually setting stack bytes a la C.
        unsafe {
            *switch_threads_context
                .as_ptr()
                .cast::<SwitchThreadsContext>() = SwitchThreadsContext::new();
        }

        new_thread.eip = entry; // !!!

        // Our thread can now be run via the `switch_threads` method.
        new_thread.status = ThreadStatus::Ready;
        new_thread
    }

    pub fn new(
        entry_instruction: NonNull<u8>,
        is_kernel: bool,
        argument: u32,
        pid: Pid,
        page_manager: PageManager,
        state: &ProcessState,
    ) -> Self {
        let tid: Tid = state.allocate_tid();

        let (kernel_stack, kernel_stack_pointer) = Self::map_stacks();

        // Create our new TCB.
        Self {
            kernel_stack_pointer,
            kernel_stack,
            eip: NonNull::new(entry_instruction.as_ptr()).expect("failed to create eip"),
            esp: NonNull::new((USER_STACK_BOTTOM_VIRT + USER_THREAD_STACK_SIZE) as *mut u8)
                .expect("failed to create esp"),
            tid,
            pid, // Potentially could be swapped to directly copy the pid of the running thread
            is_kernel,
            argument,
            status: ThreadStatus::Invalid,
            exit_code: None,
            page_manager,
        }
    }

    fn map_stacks() -> (NonNull<u8>, NonNull<u8>) {
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
        (kernel_stack, kernel_stack_pointer_top)
    }

    /// Creates the 'kernel thread'.
    ///
    /// # Safety
    /// Should only be used once while starting the threading system.
    pub fn new_kernel_thread(
        page_manager: PageManager,
        argument: u32,
        state: &ProcessState,
    ) -> Self {
        ThreadControlBlock {
            kernel_stack_pointer: NonNull::dangling(), // This will be set in the context switch immediately following.
            kernel_stack: NonNull::dangling(),
            eip: NonNull::dangling(),
            esp: NonNull::dangling(),
            tid: state.allocate_tid(),
            pid: state.allocate_pid(),
            is_kernel: true,
            argument,
            status: ThreadStatus::Running,
            exit_code: None,
            page_manager,
        }
    }

    /// If possible without stack-smashing, moves the stack pointer down and returns the new value.
    pub fn allocate_stack_space(&mut self, bytes: usize) -> Option<NonNull<u8>> {
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
            self.kernel_stack_pointer = self.kernel_stack_pointer.sub(amount);
            self.kernel_stack_pointer
        }
    }

    /// If possible without stack-smashing, moves the stack pointer down and returns the new value.
    pub fn allocate_user_stack_space(&mut self, bytes: usize) -> Option<NonNull<u8>> {
        if !self.has_user_stack_space(bytes) {
            return None;
        }

        Some(self.shift_user_stack_pointer_down(bytes))
    }

    /// Check if `bytes` bytes will fit on the kernel stack.
    const fn has_user_stack_space(&self, bytes: usize) -> bool {
        let user_bottom = USER_STACK_BOTTOM_VIRT as *const u8;

        // SAFETY: Calculates the distance between the top and bottom of the kernel stack pointers.
        let available_space = unsafe { self.esp.as_ptr().offset_from(user_bottom) as usize };

        available_space >= bytes
    }

    /// Moves the stack pointer down and returns the new position.
    fn shift_user_stack_pointer_down(&mut self, amount: usize) -> NonNull<u8> {
        // SAFETY: `has_user_stack_space` must have returned true for this amount before calling.
        unsafe {
            self.esp = self.esp.sub(amount);
            self.esp
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

unsafe impl Send for ThreadControlBlock {}
