use crate::fs::read_file;
use crate::mem::util::{
    get_cstr_from_user_space, get_slice_from_null_terminated_user_space, CStrError,
};
use crate::mem::vma::{VMAInfo, VMA};
use crate::system::{running_process, unwrap_system};
use crate::threading::process::Tid;
use crate::threading::scheduling::scheduler_yield_and_die;
use crate::threading::thread_control_block::{
    ProcessControlBlock, ThreadControlBlock, USER_HEAP_BOTTOM_VIRT,
};
use crate::user_program::elf::Elf;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::{copy_nonoverlapping, NonNull};
use kidneyos_shared::mem::OFFSET;
use kidneyos_shared::println;
use kidneyos_syscalls::{E2BIG, EFAULT, EIO, ENOENT, ENOEXEC};

const MAX_ARGUMENTS: usize = 256;

fn copy_arguments(argv: *const *const u8) -> Option<Vec<String>> {
    if argv.is_null() {
        return Some(vec![]);
    }

    let slice = unsafe { get_slice_from_null_terminated_user_space(argv, MAX_ARGUMENTS)? };

    let mut result = vec![];

    for argument in slice {
        let arg = unsafe { get_cstr_from_user_space(*argument).ok()? };

        result.push(arg.to_string());
    }

    Some(result)
}

fn move_arguments_to_stack(thread: &mut ThreadControlBlock, arguments: &[String]) -> Option<u32> {
    if arguments.is_empty() {
        return Some(0);
    }

    let argument_list_length = arguments.len() + 1;
    let argument_list_bytes = core::mem::size_of::<*const u8>() * argument_list_length;
    let argument_list = unsafe {
        core::slice::from_raw_parts_mut(
            thread
                .allocate_user_stack_space(argument_list_bytes)?
                .cast::<*const u8>()
                .as_ptr(),
            argument_list_length,
        )
    };

    argument_list[arguments.len()] = core::ptr::null();

    for (i, arg) in arguments.iter().enumerate() {
        let argument_length = arg.len() + 1;
        // Align string size to processor size.
        // We might want to later align to a larger size like 32-bytes...
        // ... if we have the courage to use SSE.
        let align = core::mem::align_of::<usize>();
        let align_correction = (argument_length % align > 0) as usize;
        let aligned_length = (argument_length / align + align_correction) * align;
        let argument_data = thread.allocate_user_stack_space(aligned_length)?;

        unsafe {
            copy_nonoverlapping(arg.as_ptr(), argument_data.as_ptr(), arg.len());

            // Add null character.
            *argument_data.add(arg.len()).as_ptr() = b'\0';
        }

        argument_list[i] = argument_data.as_ptr();
    }

    Some(argument_list.as_ptr() as u32)
}

pub fn execve(path: *const u8, argv: *const *const u8, _envp: *const *const u8) -> isize {
    let cstr = match unsafe { get_cstr_from_user_space(path) } {
        Ok(cstr) => cstr,
        Err(CStrError::Fault) => return -EFAULT,
        Err(CStrError::BadUtf8) => return -ENOENT, // ?
    };

    let Ok(data) = read_file(cstr) else {
        return -EIO;
    };

    let system = unwrap_system();

    let elf = Elf::parse_bytes(&data).ok();

    let Some(elf) = elf else { return -ENOEXEC };

    let Some(arguments) = copy_arguments(argv) else {
        return -E2BIG;
    };

    let Ok(mut thread) = ThreadControlBlock::new_from_elf(elf, 0, &system.process) else {
        return -ENOEXEC;
    };

    unsafe { thread.page_manager.load() };

    let Some(ptr) = move_arguments_to_stack(&mut thread, &arguments) else {
        return -E2BIG;
    };

    thread.argument = ptr;

    system.threads.scheduler.lock().push(Box::new(thread));

    scheduler_yield_and_die();
}

pub fn brk(heap_end: usize) -> isize {
    if heap_end >= OFFSET {
        // kernel offset
        return -1; // this is reserved
    }

    let process = running_process();
    let mut process = process.lock();

    fn get_heap(process: &mut ProcessControlBlock) -> (usize, &mut VMA) {
        // Find VMA for Heap Bottom
        let Some((addr, heap)) = process.vmas.vma_at_mut(USER_HEAP_BOTTOM_VIRT) else {
            panic!("Tried to BRK but missing heap VMA. Was this thread created properly?")
        };

        let VMAInfo::Heap = heap.info() else {
            panic!("Found VMA at heap address, but it's not the heap. Was the heap mapped?")
        };

        (addr, heap)
    }

    let (addr, heap) = get_heap(&mut process);

    let current_heap_end = addr + heap.size();

    // According to GLIBC, heap_end == 0 is treated special...
    // ...and is used to grab the end of the heap.
    if heap_end == 0 {
        return current_heap_end as isize;
    }

    if heap_end < addr {
        return -1; // ENOMEM
    }

    let new_size = heap_end - addr;

    if heap_end <= current_heap_end {
        heap.set_size(new_size); // shrink the heap
    } else {
        // addr > current_heap_end
        if !process
            .vmas
            .is_address_range_free(current_heap_end..heap_end)
        {
            return -1; // ENOMEM
        }

        // We have to grab it again, so we're able to make queries on VMAs above.
        let (_, heap) = get_heap(&mut process);

        heap.set_size(new_size);
    }

    heap_end as isize
}

// Clone only spawns threads in the same process.
// Flags are ignored
pub fn clone(
    return_eip: usize,
    _flags: u32,
    _stack: *mut u8,
    _parent_tid: *const Tid,
    _tls: u32,
    _child_tid: *const Tid,
) -> isize {
    let (pid, page_manager) = {
        let running_thread = unwrap_system().threads.running_thread.lock();

        let thread = running_thread
            .as_ref()
            .expect("Why is there no thread running?");

        (thread.pid, thread.page_manager.clone())
    };

    let child = ThreadControlBlock::new_with_page_manager(
        NonNull::new(return_eip as *mut u8).expect("Clone was executed with null eip?"),
        0,
        pid,
        page_manager,
        &unwrap_system().process,
    );

    unwrap_system()
        .threads
        .scheduler
        .lock()
        .push(Box::new(child));

    0
}
