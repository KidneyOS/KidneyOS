
pub mod context_switch;
pub mod scheduling;
pub mod thread_control_block;
pub mod thread_functions;

use crate::threading::thread_control_block::*;
use crate::threading::context_switch::*;
use crate::println;

//TEMP
pub fn test_func() {

    println!("Hello threads!");
    loop {};

}

pub fn test_halt() {

    println!("Goodbye threads!");
    loop {};

}

/**
 * To be called before any other thread functions.
 * To be called with interrupts disabled.
 */
pub fn thread_system_initialization() -> () {

    println!("Initializing Thread Sub-System...");

    // TEMP.
    let tcb_1 = ThreadControlBlock::create(test_halt);
    let tcb_2 = ThreadControlBlock::create(test_func);
    switch_threads(tcb_1, tcb_2);

    println!("Finished Thread initialization.");

}
