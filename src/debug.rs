#[allow(unused_macros)]
macro_rules! bochs_break {
    () => {
        // This is safe to use anywhere since it's a noop. The Bochs emulator
        // will break upon encountering it when magic_break: enabled=1 is
        // enabled.
        #[cfg(debug_assertions)]
        unsafe {
            core::arch::asm!("xchg bx, bx")
        }
    };
}
