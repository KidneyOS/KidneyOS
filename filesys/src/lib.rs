#![feature(linked_list_remove)] // used in structs.rs unmounting

use std::os::unix::prelude::FileExt;

use disk_device::Test;

pub mod fat;
mod disk_device;
mod structs;
mod vsfs;


