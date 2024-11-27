use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::copy_nonoverlapping;
use kidneyos_shared::println;
use kidneyos_syscalls::{E2BIG, EFAULT, EIO, ENOENT, ENOEXEC};
use crate::fs::read_file;
use crate::mem::util::{get_cstr_from_user_space, get_slice_from_null_terminated_user_space, CStrError};
use crate::system::{running_thread_ppid, unwrap_system};
use crate::threading::scheduling::scheduler_yield_and_die;
use crate::threading::thread_control_block::ThreadControlBlock;
use crate::user_program::elf::Elf;

const MAX_ARGUMENTS: usize = 256;

fn copy_arguments(argv: *const *const u8) -> Option<Vec<String>> {
    if argv.is_null() {
        return Some(vec![])
    }
    
    let slice = unsafe { get_slice_from_null_terminated_user_space(argv, MAX_ARGUMENTS)? };
    
    // println!("Got {} arguments.", slice.len());

    let mut result = vec![];
    
    for (i, argument) in slice.iter().enumerate() {
        let arg = unsafe { get_cstr_from_user_space(*argument).ok()? };

        // println!("Argument {i} is copied as {arg}.");

        result.push(arg.to_string());
    }

    Some(result)
}

fn move_arguments_to_stack(thread: &mut ThreadControlBlock, arguments: &[String]) -> Option<u32> {
    if arguments.is_empty() {
        return Some(0)
    }
    
    // println!("Copying arguments to stack.");
    let argument_list_length = arguments.len() + 1;
    let argument_list_bytes = core::mem::size_of::<*const u8>() * argument_list_length;
    // println!("Allocating {argument_list_bytes} bytes on stack, esp: {:#X} to {:#X}", thread.esp.as_ptr() as usize, thread.esp.as_ptr() as usize - argument_list_bytes);
    let argument_list = unsafe {
        core::slice::from_raw_parts_mut(
            thread.allocate_user_stack_space(argument_list_bytes)?
                .cast::<*const u8>()
                .as_ptr(),
            argument_list_length
        )
    };
    // println!("Allocated, esp is now {:#X}", thread.esp.as_ptr() as usize);

    argument_list[arguments.len()] = core::ptr::null();

    for (i, arg) in arguments.iter().enumerate() {
        println!("Argument {i} = {arg}");
        
        let argument_length = arg.len() + 1;
        // Align string size to processor size.
        // We might want to later align to a larger size like 32-bytes...
        // ... if we have the courage to use SSE.
        let align = core::mem::align_of::<usize>();
        let align_correction = (argument_length % align > 0) as usize;
        let aligned_length = (argument_length / align + align_correction) * align;
        // println!("Argument {i}, with value {arg} with bytes {aligned_length}, esp is {:#X} to {:#X}", thread.esp.as_ptr() as usize, thread.esp.as_ptr() as usize - aligned_length);
        let argument_data = thread.allocate_user_stack_space(aligned_length)?;

        // println!("Allocated arg, esp is now {:#X}", thread.esp.as_ptr() as usize);

        unsafe {
            copy_nonoverlapping(arg.as_ptr(), argument_data.as_ptr(), arg.len());

            // Add null character.
            *argument_data.add(arg.len()).as_ptr() = b'\0';
        }

        // println!("Storing {:#X} into {:#X}", argument_data.as_ptr() as usize, unsafe { argument_list.as_ptr().add(i) } as usize);
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
        return -E2BIG
    };

    let Ok(mut thread) = ThreadControlBlock::new_from_elf(elf, 0, &system.process) else {
        return -ENOEXEC
    };

    unsafe { thread.page_manager.load() };

    let Some(ptr) = move_arguments_to_stack(&mut thread, &arguments) else {
        return -E2BIG
    };
    
    thread.argument = ptr;

    system.threads.scheduler.lock().push(Box::new(thread));

    scheduler_yield_and_die();
}