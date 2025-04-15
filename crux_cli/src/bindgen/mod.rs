use std::process::Command;

use anyhow::{Context as _, Result};
use camino::Utf8Path;
use cargo_metadata::MetadataCommand;
use uniffi_bindgen::{
    bindings::{KotlinBindingGenerator, SwiftBindingGenerator},
    cargo_metadata::CrateConfigSupplier,
    library_mode, BindgenCrateConfigSupplier,
};

use crate::args::BindgenArgs;

pub fn bindgen(args: &BindgenArgs) -> Result<()> {
    let status = Command::new("cargo")
        .args(["build", "--package", &args.crate_name])
        .status()?;
    assert!(status.success());

    let library_path = Utf8Path::new("target/debug/libshared.dylib");
    let config_supplier = config_supplier()?;

    library_mode::generate_bindings(
        library_path,
        Some(args.crate_name.clone()),
        &KotlinBindingGenerator,
        &config_supplier,
        None,
        &args.out_dir.join("java"),
        true,
    )?;
    library_mode::generate_bindings(
        library_path,
        Some(args.crate_name.clone()),
        &SwiftBindingGenerator,
        &config_supplier,
        None,
        &args.out_dir.join("swift"),
        true,
    )?;
    Ok(())
}

fn config_supplier() -> Result<impl BindgenCrateConfigSupplier> {
    let mut cmd = MetadataCommand::new();
    cmd.no_deps();
    let metadata = cmd.exec().context("error running cargo metadata")?;
    Ok(CrateConfigSupplier::from(metadata))
}
