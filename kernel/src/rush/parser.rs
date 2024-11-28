use crate::rush::cd::cd;
use crate::rush::clear::clear;
use crate::rush::env::CURR_DIR;
use crate::rush::ls::ls_config::LsConfig;
use crate::rush::ls::ls_core::list;
use alloc::string::ToString;
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
            cd(args);
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
            let curr_dir = CURR_DIR.read().to_string();
            list(curr_dir.as_ref(), config);
        }
        "pwd" => {
            // print working directory
            println!("{}", CURR_DIR.read().as_str());
        }
        _ => {
            // command not found
            eprintln!("rush: {}: command not found", command);
        }
    }
}
