use crate::println;

type TID = u16;


// Used for specifying any or no thread.
enum TIDOptions {
    Valid(TID),
    Invalid,
    Any
}

enum ThreadStatus {
    Invalid,
    Running,
    Ready,
    Blocked,
}

pub struct ThreadControlBlock {

    tid: TID,
    status: ThreadStatus

}

pub fn thread_system_initialization() -> () {

    println!("Initializing Thread Sub-System...");

    println!("Finished Thread initialization.");

}
