use std::process::Command;

use anyhow::{anyhow, bail, Context as _, Result};
use camino::Utf8PathBuf;
use cargo_metadata::{Metadata, MetadataCommand};
use uniffi_bindgen::{
    bindings::{KotlinBindingGenerator, SwiftBindingGenerator},
    cargo_metadata::CrateConfigSupplier,
    library_mode,
};

use crate::args::BindgenArgs;

/// Generate FFI bindings using uniffi
///
/// # Errors
/// if we cannot get cargo metadata, run a cargo build, or generate the bindings
pub(crate) fn bindgen(args: &BindgenArgs) -> Result<()> {
    let crate_name = &args.crate_name;

    let mut cmd = MetadataCommand::new();
    cmd.no_deps();
    let metadata = cmd.exec().context("running cargo metadata")?;

    let library_path = find_library_path(&metadata, crate_name).context("finding library path")?;

    let config_supplier = CrateConfigSupplier::from(metadata);

    if !Command::new("cargo")
        .args(["build", "--package", crate_name])
        .status()
        .context("running cargo build")?
        .success()
    {
        bail!("cargo build failed");
    }

    library_mode::generate_bindings(
        &library_path,
        None,
        &KotlinBindingGenerator,
        &config_supplier,
        None,
        &args.out_dir.join("java"),
        true,
    )
    .context("generating Kotlin bindings")?;
    library_mode::generate_bindings(
        &library_path,
        None,
        &SwiftBindingGenerator,
        &config_supplier,
        None,
        &args.out_dir.join("swift"),
        true,
    )
    .context("generating Swift bindings")?;
    Ok(())
}

fn find_library_path(metadata: &Metadata, crate_name: &String) -> Result<Utf8PathBuf> {
    let library_name = metadata
        .workspace_packages()
        .iter()
        .find(|package| &package.name == crate_name)
        .ok_or_else(|| anyhow!(r#"crate "{}" not found"#, crate_name))?
        .targets
        .iter()
        .find(|target| target.is_lib())
        .ok_or_else(|| anyhow!(r#"crate "{}" has no lib target"#, crate_name))?
        .name
        .clone();
    let target_dir = &metadata.target_directory;
    let library_path = &target_dir.join(format!("debug/lib{library_name}"));
    let library_path = ["rlib", "dylib", "a"]
        .iter()
        .map(|&ext| {
            let mut path = library_path.clone();
            path.set_extension(ext);
            path
        })
        .find(|path| path.exists())
        .ok_or_else(|| anyhow!(r#"library "{library_path}" not found"#))?;
    Ok(library_path)
}
