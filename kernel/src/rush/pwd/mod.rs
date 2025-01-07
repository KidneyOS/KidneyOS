use crate::system::running_process;
use kidneyos_shared::println;

pub fn pwd() {
    let curr_path = running_process().lock().cwd_path.clone();
    println!("{}", curr_path);
}
