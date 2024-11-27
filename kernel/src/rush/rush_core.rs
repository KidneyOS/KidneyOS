use crate::rush::env::{CURR_DIR, HOST_NAME};
use crate::rush::parser::parse_input;
use crate::sync::mutex::Mutex;
use crate::system::unwrap_system;
use crate::threading::scheduling::scheduler_yield_and_continue;
use alloc::string::String;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::SeqCst;
use kidneyos_shared::print;
use kidneyos_shared::video_memory::VIDEO_MEMORY_WRITER;

pub static IS_SYSTEM_FULLY_INITIALIZED: AtomicBool = AtomicBool::new(false);

static BUFFER: Mutex<String> = Mutex::new(String::new());
static JUST_READ_LINE: AtomicBool = AtomicBool::new(false);

pub extern "C" fn rush_loop() -> ! {
    // initialize RUSH ----------------------------------------------------------------------------
    unwrap_system()
        .input_buffer
        .lock()
        .on_receive
        .insert(0, |input| {
            BUFFER.lock().push(input as char);

            if input == 0x08 || input == 0x7F {
                // BS (Backspace) or DEL (Delete)
                let mut buffer = BUFFER.lock();
                buffer.pop(); // BS or DEL

                // Remove the previous character
                if !buffer.is_empty() {
                    buffer.pop();
                    unsafe { VIDEO_MEMORY_WRITER.backspace() };
                }
            } else if input != b'\r' {
                print!("{}", input as char);
            } else {
                print!("\n");
                JUST_READ_LINE.store(true, SeqCst);
            }
        });

    // Wait until the system is fully initialized to avoid weird display issues
    while !IS_SYSTEM_FULLY_INITIALIZED.load(SeqCst) {
        scheduler_yield_and_continue();
    }

    print_prompt(false);
    loop {
        if JUST_READ_LINE.load(SeqCst) {
            let mut buffer = BUFFER.lock();
            buffer.pop(); // remove the newline character
            parse_input(&buffer); // parse and execute the command
            buffer.clear(); // clear the buffer
            JUST_READ_LINE.store(false, SeqCst);

            print_prompt(false);
        }

        scheduler_yield_and_continue(); // Until we can read input
    }
}

fn print_prompt(is_root: bool) {
    let curr_dir = CURR_DIR.read();
    let host_name = HOST_NAME.read();

    print!("{}:{}", host_name.as_str(), curr_dir.as_str());

    if is_root {
        print!("# ");
    } else {
        print!("$ ");
    }
}
