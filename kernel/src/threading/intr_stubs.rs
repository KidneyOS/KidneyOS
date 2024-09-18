pub type IntrStubFunc = fn();

pub static mut INTR_STUBS: [IntrStubFunc; 256] = [intr_exit; 256];

pub fn intr_exit() {
    panic!("Unhandled interrupt");
}
