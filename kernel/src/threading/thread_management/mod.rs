use alloc::boxed::Box;

mod thread_manager;
mod thread_manager_128;

pub use thread_manager::ThreadManager;
pub use thread_manager_128::ThreadManager128;

pub static mut THREAD_MANAGER: Option<Box<dyn ThreadManager>> = None;

pub fn initialize_thread_manager() {
    unsafe {
        THREAD_MANAGER = Some(Box::new(ThreadManager128::new()));
    }
}
