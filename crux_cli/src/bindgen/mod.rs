mod cargo_metadata;

use std::process::Command;

use ::cargo_metadata::MetadataCommand;
use anyhow::{Context as _, Result};
use camino::Utf8PathBuf;
use uniffi::{KotlinBindingGenerator, SwiftBindingGenerator};

use crate::args::BindgenArgs;
use cargo_metadata::CrateConfigSupplier;

pub fn bindgen(args: &BindgenArgs) -> Result<()> {
    let status = Command::new("cargo")
        .args(["build", "--package", &args.crate_name])
        .status()?;
    assert!(status.success());

    let library_path = Utf8PathBuf::from("target/debug/libshared.dylib");
    let config_supplier = config_supplier()?;

    uniffi::generate_bindings_library_mode(
        &library_path,
        Some(args.crate_name.clone()),
        &KotlinBindingGenerator,
        &config_supplier,
        None,
        &args.out_dir.join("java"),
        true,
    )?;
    uniffi::generate_bindings_library_mode(
        &library_path,
        Some(args.crate_name.clone()),
        &SwiftBindingGenerator,
        &config_supplier,
        None,
        &args.out_dir.join("swift"),
        true,
    )?;
    Ok(())
}

fn config_supplier() -> Result<impl uniffi_bindgen::BindgenCrateConfigSupplier> {
    let mut cmd = MetadataCommand::new();
    cmd.no_deps();
    let metadata = cmd.exec().context("error running cargo metadata")?;
    Ok(CrateConfigSupplier::from(metadata))
}
