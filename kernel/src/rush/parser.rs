use crate::rush::clear::clear;
use crate::rush::ls::ls_config::LsConfig;
use crate::rush::ls::ls_core::list;
use crate::rush::rush_core::CURRENT_DIR;
use alloc::vec::Vec;
use kidneyos_shared::{eprintln, println};
use kidneyos_syscalls::exit;

pub(crate) fn parse_input(input: &str) {
    let mut tokens = input.split_whitespace();
    let command = tokens.next().unwrap_or("");
    let args = tokens.collect::<Vec<&str>>();

    match command {
        "cat" => {
            // print the contents of a file
        }
        "cd" => {
            // change directory
        }
        "clear" => {
            // clear the screen
            clear();
        }
        "echo" => {
            // print the arguments
        }
        "exit" => {
            exit(0);
        }
        "ls" => {
            let config = LsConfig::from_args(args);
            list(CURRENT_DIR.lock().as_ref(), config);
        }
        "pwd" => {
            // print working directory
            println!("{}", CURRENT_DIR.lock());
        }
        _ => {
            // command not found
            eprintln!("rush: {}: command not found", command);
        }
    }
}
