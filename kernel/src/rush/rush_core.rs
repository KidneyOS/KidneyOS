use crate::rush::parser::parse_input;
use crate::sync::mutex::Mutex;
use crate::system::unwrap_system;
use crate::threading::scheduling::scheduler_yield_and_continue;
use alloc::string::String;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::SeqCst;
use kidneyos_shared::print;

pub static CURRENT_DIR: Mutex<&str> = Mutex::new("/");

static BUFFER: Mutex<String> = Mutex::new(String::new());
static JUST_READ_LINE: AtomicBool = AtomicBool::new(false);

pub extern "C" fn rush_loop() -> ! {
    unwrap_system()
        .input_buffer
        .lock()
        .on_receive
        .insert(0, |input| {
            BUFFER.lock().push(input as char);

            if input != b'\r' {
                print!("{}", input as char);
            } else {
                print!("\n");
                JUST_READ_LINE.store(true, SeqCst);
            }
        });

    print!("> ");
    loop {
        if JUST_READ_LINE.load(SeqCst) {
            let mut buffer = BUFFER.lock();
            buffer.pop(); // remove the newline character
            parse_input(&buffer); // parse and execute the command
            buffer.clear(); // clear the buffer
            JUST_READ_LINE.store(false, SeqCst);

            print!("> ");
        }

        scheduler_yield_and_continue(); // Until we can read input
    }
}
