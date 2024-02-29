use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    if env::var("CARGO_CFG_TARGET_OS")? != "none" {
        return Ok(());
    }

    println!("cargo:rustc-link-search=native=build/trampoline");
    println!("cargo:rustc-link-lib=static=kidneyos_trampoline");
    println!("cargo:rerun-if-changed=build/trampoline/libkidneyos_trampoline.a");

    Ok(())
}
