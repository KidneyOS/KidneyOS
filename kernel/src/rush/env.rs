use crate::sync::rwlock::sleep::RwLock;
use alloc::string::{String, ToString};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref CURR_DIR: RwLock<String> = RwLock::new("/".to_string());
    pub static ref HOST_NAME: RwLock<String> = RwLock::new("kidney".to_string());
}
