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
pub struct Process;
const PGSIZE: usize = 4096;
// Assuming we have a module `thread` that provides similar functionalities
// and `Tid` is a type alias for thread ID, with `TID_ERROR` being a constant for an error state.
// `palloc_get_page` and `strlcpy` would need to be safe Rust functions or wrapped in unsafe blocks if they directly interface with C functions or system calls.
// `start_process` needs to be a function pointer or a closure that matches the expected signature.

pub fn process_execute(file_name: &str) -> Tid {
    // Allocate memory for the filename copy.
    // In Rust, we don't generally deal with raw pointers for memory allocation, instead we use Vec<u8> or String.
    let mut fn_copy = vec![0u8; PGSIZE];

    // Rust strings already ensure null-termination, so we simply copy the bytes.
    // We need to handle potential UTF-8 encoding issues since Rust strings are UTF-8.
    let bytes_to_copy = file_name.as_bytes().len().min(PGSIZE - 1); // Leave space for null-terminator
    fn_copy[..bytes_to_copy].copy_from_slice(file_name.as_bytes());
    fn_copy[bytes_to_copy] = 0; // Null-terminate the string

    // In Rust, we would pass a closure that captures `fn_copy` to the new thread.
    // `thread_create` should be a safe wrapper around the actual thread creation.
    let tid = thread_create(move || start_process(&fn_copy));

    if tid == TID_ERROR {
        // In Rust, the memory would be automatically freed once `fn_copy` goes out of scope.
    }

    tid
}

pub unsafe fn start_process(file_name_: *mut u8) -> ! {
    let file_name = file_name_;

    // TODO: need to implement interrupt frame
    let mut if_: IntrFrame = std::mem::zeroed();
    if_.gs = SEL_UDSEG;
    if_.fs = SEL_UDSEG;
    if_.es = SEL_UDSEG;
    if_.ds = SEL_UDSEG;
    if_.ss = SEL_UDSEG;
    if_.cs = SEL_UCSEG;
    if_.eflags = FLAG_IF | FLAG_MBS;

    let success = load(file_name, &mut if_.eip, &mut if_.esp);

    // If load failed, free the allocated page and exit the thread.
    // TODO: need to implement palloc
    palloc_free_page(file_name);
    if !success {
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

