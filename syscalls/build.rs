use std::{env, fs};

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR was not defined, we need this to define C headers");

    println!("cargo::rerun-if-changed=bindgen.toml");
    println!("cargo::rerun-if-changed=src");

    let config = cbindgen::Config::from_file("bindgen.toml")
        .expect("Failed to read bindgen.toml (the bindgen config)");

    fs::create_dir("include").ok();

    cbindgen::Builder::new()
        .with_config(config)
        .with_crate(crate_dir)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("include/kidneyos.h");
}
