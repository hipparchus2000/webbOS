use std::env;
use std::path::PathBuf;

fn main() {
    // Get the directory containing the Cargo.toml
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
    // Path to our linker script
    let linker_script = PathBuf::from(&manifest_dir).join("linker.ld");
    
    // Tell cargo to pass the linker script to rust-lld
    println!("cargo:rustc-link-arg=-T{}", linker_script.display());
    
    // Tell cargo to rerun if the linker script changes
    println!("cargo:rerun-if-changed={}", linker_script.display());
}
