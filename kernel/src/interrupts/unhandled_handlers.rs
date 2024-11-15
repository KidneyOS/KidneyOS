use core::arch::{asm, global_asm};

extern "C" {
    fn unhandled_handlers();
}

#[naked]
pub unsafe extern "C" fn unhandled_handler_inner() -> ! {
    extern "C" fn inner(num: u32) -> ! {
        panic!("unhandled interrupt {num:#x}");
    }

    asm!(
        // compute interrupt number from return address, which was pushed
        // to the stack by call instruction.
        "
        // interrupt number = (return address - unhandled_handlers) / 5
        // (5 bytes = length of call instruction)
        mov eax, [esp]
        lea ebx, {}
        sub eax, ebx
        xor edx, edx
        mov ecx, 5 
        div ecx
        // in x86, the return address points to the instruction *after* the call
        // so we need to subtract 1 to get the actual interrupt value
        dec eax
        push eax
        call {}
        ",
        sym unhandled_handlers,
        sym inner,
        options(noreturn),
    )
}

macro_rules! repeat4 {
    ($x:expr) => {
        concat!($x, $x, $x, $x)
    };
}

global_asm!(concat!("
unhandled_handlers:
", 
// Repeat `call unhandled_handler_inner` 256 times. It's ugly but it works.
// Unfortunately there seems to be no way of directly getting the interrupt number inside
// the interrupt handler.
repeat4!(repeat4!(repeat4!(repeat4!("call {0}\n"))))),
    sym unhandled_handler_inner);

/// Get pointer to function which panicks with message "unhandled interrupt i"
pub fn get_unhandled_handler(i: u8) -> usize {
    // 5 bytes = length of call instruction
    (unhandled_handlers as usize) + usize::from(i) * 5
}
