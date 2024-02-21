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
        *esp = *esp & 0xffff_fffc;

        // Push a null sentinel (end of arguments marker).
        // TODO: type usize cannot be dereferenced, need to fix this.
        *esp = *esp.wrapping_sub(4);
        *(esp as *mut usize as *mut u32) = 0;

        // Push argv pointers in reverse order.
        for &arg in argv.iter().rev() {
            *esp = *esp.wrapping_sub(4);
            *(esp as *mut usize as *mut usize) = arg;
        }

        // Push the address of argv[0].
        *esp = *esp.wrapping_sub(4);
        *(esp as *mut usize as *mut usize) = *esp.wrapping_add(4);

        // Push argc.
        *esp = *esp.wrapping_sub(4);
        *(esp as *mut usize as *mut usize) = argc;

        // Push a fake return address.
        *esp = *esp.wrapping_sub(4);
        *(esp as *mut usize as *mut u32) = 0;
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
        // Our implementation for Task 1:
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
        pagedir_activate(t.pagedir);

        // Update the TSS with the thread's kernel stack.
        tss_update();
    }
}

