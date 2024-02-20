
use crate::threading::ThreadControlBlock;
use crate::println;

#[repr(C, packed)]
pub struct StackFrame {

    // // General Registers.
    // pub eax: usize, // Accumulator.
    // pub ebx: usize, // Base (for memory access).
    // pub ecx: usize, // Counter (for loops).
    // pub edx: usize, // Data.

    // // Indexes & pointers.
    // pub edi: usize, // Destination index.
    // pub esi: usize, // Source index.
    // pub ebp: usize, // Stack base pointer.
    // pub esp: usize, // Stack pointer.
    // pub eip: usize  // Index pointer.

    // pub edi: usize, // Destination index.
    // pub esi: usize, // Source index.
    // pub ebp: usize, // Stack base pointer.
    // pub ebx: usize, // Base (for memory access).
    // pub eip: usize  // Index pointer.

    // Manually pushed.
    pub eax: usize,
    pub ebx: usize,
    pub ecx: usize,
    pub edx: usize,
    pub esi: usize,
    pub edi: usize,
    pub ebp: usize,

    // Automatically pushed by cpu
    pub eip: usize,
    pub cs: usize,
    pub eflags: usize,
    pub esp: usize,
    pub ss: usize,

}


pub fn thread_switch(switch_from: ThreadControlBlock, switch_to: ThreadControlBlock) {

    // TEMP.
    // switch_from should not need to be passed in
    // Safety checks needed.

    let x;
    unsafe { x = switch_to.stack_pointer.as_ptr().cast::<usize>(); }
    println!("{:?} : {:?}", switch_from.stack_pointer.as_ptr().cast::<usize>(), x);

    unsafe {
        context_switch(
        switch_from.stack_pointer.as_ptr().cast::<usize>(),
        switch_to.stack_pointer.as_ptr().cast::<usize>()
        );
    }

}

unsafe fn context_switch(_previous_stack_pointer: *mut usize, _next_stack_pointer: *mut usize) {

    // Our function arguments are conviently placed in edi and esi for us.
    core::arch::asm!(
        r#"
        push ebp
        push edi
        push esi
        push edx
        push ecx
        push ebx
        push eax

        mov [edi], esp
        mov esp, esi

        push eax
        push ebx
        push ecx
        push edx
        push esi
        push edi
        push ebp
        "#,
        options(noreturn)
    )

}
