use std::{env, fs, path::PathBuf};

fn main() {
    // Put memory.x where the linker (cortex-m-rt's link.x) finds it.
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::copy("memory.x", out.join("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
}
