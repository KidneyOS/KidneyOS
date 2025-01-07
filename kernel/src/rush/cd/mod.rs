use crate::system::{root_filesystem, running_process};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use kidneyos_shared::eprintln;

pub fn cd(args: Vec<&str>) {
    let path: String;

    // TODO: change to home directory
    let home_dir = "/";

    if args.is_empty() {
        path = home_dir.to_string();
    } else if args.len() > 1 {
        // Too many arguments
        eprintln!("rush: cd: too many arguments");
        return;
    } else {
        let new_path = args[0].to_string();
        let cd_path: String;

        if new_path.starts_with('/') {
            // Absolute path
            cd_path = new_path.to_string();
        } else if let Some(stripped) = new_path.strip_prefix("~/") {
            // Home directory
            cd_path = home_dir.to_string() + stripped;
        } else {
            // Relative path
            let curr_path = running_process().lock().cwd_path.clone();
            // let curr_path = "/".to_string();
            // curr_path.push('/'); // Pad with a slash to avoid edge cases
            cd_path = curr_path + &new_path;
        }

        // Resolve the new path
        let ret = resolve_path(cd_path.clone());
        if ret.is_err() {
            eprintln!("rush: cd: {}: No such file or directory", new_path);
            return;
        }
        path = ret.unwrap();
    }

    // Change the directory to the new path
    // eprintln!("DEBUG | cd | {}", path);
    let running = running_process();
    let mut pcb = running.lock();

    root_filesystem()
        .lock()
        .chdir(&mut pcb, &path)
        .unwrap_or_else(|_| {
            eprintln!("rush: cd: No such file or directory");
        });
}

fn resolve_path(path: String) -> Result<String, String> {
    let error = Err("No such file or directory".to_string());

    let mut resolved_path = String::new();
    let mut resolved_parts = Vec::new();

    let path_parts = path.split('/').collect::<Vec<&str>>();
    for part in path_parts {
        match part {
            "" => {
                // Skip empty parts
            }
            "." => {
                // Skip current directory
            }
            ".." => {
                // Go up one directory
                if !resolved_parts.is_empty() {
                    // Not at root directory
                    resolved_parts.pop();
                }
            }
            _ => {
                // Check if special characters are present
                if part.contains('~') {
                    return error;
                }
                resolved_parts.push(part);
            }
        }
    }

    resolved_path.push('/');
    for part in resolved_parts {
        resolved_path.push_str(part);
        resolved_path.push('/');
    }

    Ok(resolved_path)
}
