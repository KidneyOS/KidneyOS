// Assuming we have a module called `threads` that provides the `Thread` struct and related functions.
// Similarly, we would need modules for `gdt`, `pagedir`, `tss`, `filesys`, and any other C modules used.

use crate::threads::{self, Thread, Tid};
use crate::gdt;
use crate::pagedir::pagedir_activate;
use crate::pagedir::pagedir_destroy;
use crate::tss;
use core::ptr;
use std::ffi::CString;
use std::os::raw::c_void;
use core::arch::asm;
use core::{
    alloc::{AllocError, Allocator, Layout},
    mem::size_of,
    ptr::NonNull,
};
pub struct Process;
const PGSIZE: usize = 4096;
const TID_ERROR: i32 = -1;
// Assuming we have a module `thread` that provides similar functionalities
// and `Tid` is a type alias for thread ID, with `TID_ERROR` being a constant for an error state.
// `start_process` needs to be a function pointer or a closure that matches the expected signature.

fn push_argument(esp: &mut usize, argc: usize, argv: &[usize]) {
    unsafe {
        // Align the stack pointer to a 4-byte boundary.
        *esp &= 0xFFFF_FFFC;

        // Push a null sentinel (end of arguments marker).
        *esp = (*esp).wrapping_sub(4);
        *((*esp) as *mut u32) = 0;

        // Push argv pointers in reverse order.
        for &arg in argv.iter().rev() {
            *esp = (*esp).wrapping_sub(4);
            *((*esp) as *mut usize) = arg;
        }

        // Push the address of argv[0]. This is tricky in Rust due to ownership rules,
        // but since we're dealing with raw pointers here, we simulate it:
        *esp = (*esp).wrapping_sub(4);
        *((*esp) as *mut usize) = *esp + 4;

        // Push argc.
        *esp = (*esp).wrapping_sub(4);
        *((*esp) as *mut usize) = argc;

        // Push a fake return address.
        *esp = (*esp).wrapping_sub(4);
        *((*esp) as *mut u32) = 0;
    }
}
pub fn process_execute(file_name: &str) -> Tid {
    // Allocate memory for the filename copy.
    // In Rust, we don't generally deal with raw pointers for memory allocation, instead we use Vec<u8> or String.
    let mut fn_copy = vec![0u8; PGSIZE];

    // allocate and copy string
    let mut fn_copy = file_name.to_owned();
    let mut fn_copy2 = file_name.to_owned();


    let first_word = fn_copy2.split_whitespace().next().unwrap_or_default().to_string();// Null-terminate the string

    let tid = thread_create(&first_word, PRI_DEFAULT, start_process, &fn_copy);

    if tid == TID_ERROR {
        // In Rust, the memory would be automatically freed once `fn_copy` goes out of scope.
        return TID_ERROR;
    }

    // Simulate waiting on a semaphore with a hypothetical `sema_down` function
    // Assume `thread_current()` is replaced with a Rust equivalent that accesses the current thread context
    sema_down(&thread_current().sema);
    if !thread_current().success {
        return TID_ERROR;
    }

    tid
}

pub unsafe fn start_process(file_name_: *mut u8) -> ! {
    let mut success = false;

    // In Rust, instead of using `malloc` and `strlcpy`, you can clone the string directly.
    let fn_copy = file_name_.to_owned();

    // Initialize interrupt frame, TODO: we still need to implement the interrupt frame
    let mut if_ = IntrFrame {
        gs: SEL_UDSEG,
        fs: SEL_UDSEG,
        es: SEL_UDSEG,
        ds: SEL_UDSEG,
        ss: SEL_UDSEG,
        cs: SEL_UCSEG,
        eflags: FLAG_IF | FLAG_MBS,
        eip: 0,
        esp: 0,
    };

    // Simulate extracting the first token from the string (similar to `strtok_r` in C).
    let file_name = file_name_.split_whitespace().next().unwrap_or_default();

    // TODO: implement load in process.rs
    success = load(file_name, &mut if_.eip, &mut if_.esp);

    if success {
        // Calculate the number of parameters and the specification of parameters.
        let mut argc = 0;
        let mut argv = [0usize; 50]; // Use usize for pointers/addressing.
        let mut esp = if_.esp;

        // Split `fn_copy` into tokens, equivalent to the strtok_r loop in C.
        for token in fn_copy.split_whitespace() {
            let token_len = token.len() + 1; // +1 for null terminator, assuming we need to simulate C-style strings.
            esp -= token_len;

            // Assuming we have a way to copy the token directly to the simulated stack at `esp`.
            // This would require unsafe pointer manipulation in Rust.
            unsafe {
                core::ptr::copy_nonoverlapping(token.as_ptr(), esp as *mut u8, token_len);
            }

            argv[argc] = esp;
            argc += 1;

            if argc >= 50 { break; } // Ensuring we don't exceed the argv array bounds.
        }

        // Simulate pushing arguments onto the stack.
        push_argument(&mut esp, argc, &argv);

        // Adjust the interrupt frame's stack pointer.
        if_.esp = esp;

        // Record the exec_status of the parent thread's success and sema up parent's semaphore.
        // Assuming `thread_current` and `sema_up` are available or appropriately simulated.
        let current_thread = thread_current();
        current_thread.parent.success = true;
        sema_up(&current_thread.parent.sema);
    }

    // If load failed
    if !success {
        // Record the exec_status of the parent thread's success and sema up parent's semaphore.
        let current_thread = thread_current();
        current_thread.parent.success = false;
        sema_up(&current_thread.parent.sema);
    
        // Assuming `thread_exit` is a function that terminates the current thread.
        thread_exit();
    }
    // Start the user process by jumping to the interrupt exit code.
    // This is highly architecture and OS specific, and typically wouldn't be done in Rust.
    asm!("movl $0, %esp; jmp intr_exit" : : "g"(&if_) : "memory");

    // `NOT_REACHED` is typically a macro to indicate the program should not reach this point.
    // In Rust, we can use `unreachable!()` to indicate that this point in the code should never be reached.
    unreachable!();
}

pub fn process_wait(child_tid: tid_t) -> Result<i32, &'static str> {
    // Implementation for student
    // Since the real implementation is not provided, we will return an error.
    Err("Not implemented")
}

fn process_exit() {
    unsafe {
        let cur = thread_current(); // Assuming `thread_current` returns a mutable reference to the current thread.
        
        // TODO: thread must have page directory field
        if let Some(pd) = cur.pagedir.take() {
            // Set the current thread's page directory to null before switching page directories.
            pagedir_activate(std::ptr::null_mut());
            // Destroy the process's page directory.
            pagedir_destroy(pd);
        }
    }
}

fn process_activate() {
    unsafe {
        // Get the current thread. We're assuming that `thread_current` is a function that returns a mutable reference to the current thread.
        let t = thread_current();

        // Activate the thread's page tables.
        // The `pagedir` field is assumed to be a pointer to the page directory, so we dereference it here.
        pagedir_activate(t.pagedir); //this function loads page directory PD into the CPU's page directory base register.

        // Update the TSS with the thread's kernel stack. Sets the ring 0 stack pointer in the TSS to point to the end of the thread stack. 
        tss_update();
    }
}

// Define the ELF data structures in Rust.
#[repr(C)]
#[derive(Debug, Default)]
struct Elf32Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u32,
    e_phoff: u32,
    e_shoff: u32,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Debug, Default)]
struct Elf32Phdr {
    p_type: u32,
    p_offset: u32,
    p_vaddr: u32,
    p_paddr: u32,
    p_filesz: u32,
    p_memsz: u32,
    p_flags: u32,
    p_align: u32,
}

fn load(file_name: &str, eip: &mut Option<fn()>, esp: &mut Option<*mut u8>) -> Result<bool, io::Error> {
    let t = Thread::current(); // Assuming a method to get the current thread context
    let mut ehdr = Elf32Ehdr::default();
    let mut file = None;
    let mut file_ofs: i64;
    let mut success = false;

    // Allocate and activate page directory
    t.pagedir = pagedir_create(); // Creates a new page directory that has mappings for kernel virtual addresses, but none for user virtual addresses. Returns the new page directory, or a null pointer if memory allocation fails. 
    if t.pagedir.is_none() {
        return Ok(false);
    }
    process_activate();

    // Open executable file
    acquire_lock_f(); // lock the process using a lock to do file operation.
    let mut file_handle = Filesys::open(Path::new(file_name))?; // need file system implementation, rn we are using std file system.
    if file_handle.is_none() {
        println!("load: {}: open failed", file_name);
        return Ok(false);
    }
    let file_handle = file_handle.unwrap(); // Safe to unwrap here due to the check above

    // Deny write for the opened file, need to implement in file system.
    file_deny_write(&file_handle);
    t.file_owned = Some(file_handle.try_clone()?); // Assuming Filesys::open returns a File and File has try_clone()

    // Read and verify executable header
    let mut ehdr_buf = vec![0u8; std::mem::size_of::<Elf32_Ehdr>()];
    if file_handle.read_exact(&mut ehdr_buf)? != ehdr_buf.len()
        || !ehdr_buf.starts_with(b"\x7fELF\x01\x01\x01")
        || ehdr.e_type != 2
        || ehdr.e_machine != 3
        || ehdr.e_version != 1
        || ehdr.e_phentsize != std::mem::size_of::<Elf32_Phdr>() as u16
        || ehdr.e_phnum > 1024 {
            println!("load: {}: error loading executable", file_name);
            return Ok(false);
        }

    // Read program headers
    file_ofs = ehdr.e_phoff as i64;
    for i in 0..ehdr.e_phnum {
        let mut phdr = Elf32_Phdr::default();

        if file_ofs < 0 || file_ofs as u64 > file_handle.metadata()?.len() {
            return Ok(false);
        }
        file_handle.seek(SeekFrom::Start(file_ofs as u64))?;

        let mut phdr_buf = vec![0u8; std::mem::size_of::<Elf32_Phdr>()];
        if file_handle.read_exact(&mut phdr_buf)? != phdr_buf.len() {
            return Ok(false);
        }
        file_ofs += std::mem::size_of::<Elf32_Phdr>() as i64;

        match phdr.p_type {
            PT_LOAD => {
                if validate_segment(&phdr, &file_handle) {
                    let writable = (phdr.p_flags & PF_W) != 0;
                    let file_page = phdr.p_offset & !(PGMASK as u32);
                    let mem_page = phdr.p_vaddr & !(PGMASK as u32);
                    let page_offset = phdr.p_vaddr & PGMASK as u32;
                    let (read_bytes, zero_bytes) = if phdr.p_filesz > 0 {
                        // Normal segment
                        let read_bytes = page_offset + phdr.p_filesz;
                        let zero_bytes = ROUND_UP(page_offset + phdr.p_memsz, PGSIZE as u32) - read_bytes;
                        (read_bytes, zero_bytes)
                    } else {
                        // Entirely zero
                        let read_bytes = 0;
                        let zero_bytes = ROUND_UP(page_offset + phdr.p_memsz, PGSIZE as u32);
                        (read_bytes, zero_bytes)
                    };
                    if !load_segment(&file_handle, file_page, mem_page, read_bytes, zero_bytes, writable) {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }
            _ => {} // Ignoring other segment types
        }
    }

    Ok(true) // If all operations succeeded
}