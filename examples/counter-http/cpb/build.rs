use std::{env, fs, path::PathBuf};

fn main() {
    // The SoftDevice version of the board decides where the application starts in flash.
    let v6 = env::var_os("CARGO_FEATURE_SD_V6").is_some();
    let v7 = env::var_os("CARGO_FEATURE_SD_V7").is_some();
    let layout = match (v6, v7) {
        (true, false) => "memory-s140-v6.x",
        (false, true) => "memory-s140-v7.x",
        _ => panic!(
            "enable exactly one of the features `sd-v6` (the default) and `sd-v7`; \
             for v7 build with `--no-default-features --features sd-v7`"
        ),
    };

    // Put the chosen layout where the linker (cortex-m-rt's link.x) finds it, as memory.x.
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::copy(layout, out.join("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory-s140-v6.x");
    println!("cargo:rerun-if-changed=memory-s140-v7.x");
}
