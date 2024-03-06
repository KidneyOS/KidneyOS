use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    if env::var("CARGO_CFG_TARGET_OS")? != "none" {
        return Ok(());
    }

    let target_dir = env::var("CARGO_TARGET_DIR")?;

    println!("cargo:rustc-link-search=native={target_dir}/../trampoline");
    println!("cargo:rustc-link-lib=static=kidneyos_trampoline");
    println!("cargo:rerun-if-changed={target_dir}/../trampoline/libkidneyos_trampoline.a");

    Ok(())
}
