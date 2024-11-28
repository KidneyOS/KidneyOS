use crate::rush::env::CURR_DIR;
use alloc::string::ToString;
use alloc::vec::Vec;
use kidneyos_shared::eprintln;

// TODO: Use [`Path`]
pub fn cd(args: Vec<&str>) {
    if args.is_empty() {
        // TODO: change to home directory
        let mut curr_dir = CURR_DIR.write();
        *curr_dir = "/".to_string();
        return;
    }

    if args.len() > 1 {
        eprintln!("rush: cd: too many arguments");
        return;
    }

    let new_path = args[0];

    if new_path.starts_with('/') {
        let mut curr_dir = CURR_DIR.write();
        *curr_dir = new_path.to_string();
    } else {
        let mut curr_dir = CURR_DIR.write();
        let mut path = curr_dir.clone();
        path.push_str(new_path);
        if !path.ends_with('/') {
            path.push('/');
        }
        *curr_dir = path;
    }
}
