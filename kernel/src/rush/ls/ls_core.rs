use crate::rush::ls::ls_config::LsConfig;
use kidneyos_shared::println;

pub fn list(dir: &str, config: LsConfig) {
    println!("Listing directory: {}", dir);
    println!("Config: {}", config);
}
