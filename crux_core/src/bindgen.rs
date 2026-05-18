use anyhow::{Context as _, Result, anyhow};
use camino::Utf8PathBuf;
use cargo_metadata::{Metadata, MetadataCommand};
use uniffi_bindgen::{
    bindings::KotlinBindingGenerator, cargo_metadata::CrateConfigSupplier, library_mode,
};

/// Generate Kotlin FFI bindings using uniffi for the given crate, writing output to `out_dir`.
///
/// # Errors
/// Returns an error if cargo metadata cannot be run, the library cannot be found, or
/// the bindings cannot be generated.
pub fn bindgen_kotlin(crate_name: &str, out_dir: &std::path::Path) -> Result<()> {
    let mut cmd = MetadataCommand::new();
    cmd.no_deps();
    let metadata = cmd.exec().context("running cargo metadata")?;

    let library_path = find_library_path(&metadata, crate_name).context("finding library path")?;

    let config_supplier = CrateConfigSupplier::from(metadata);

    library_mode::generate_bindings(
        &library_path,
        None,
        &KotlinBindingGenerator,
        &config_supplier,
        None,
        &Utf8PathBuf::from_path_buf(out_dir.to_path_buf())
            .map_err(|p| anyhow!("path {} has non-unicode characters", p.display()))?,
        true,
    )
    .context("generating Kotlin bindings")?;

    Ok(())
}

fn find_library_path(metadata: &Metadata, crate_name: &str) -> Result<Utf8PathBuf> {
    let library_name = metadata
        .workspace_packages()
        .iter()
        .find(|package| package.name == crate_name)
        .ok_or_else(|| anyhow!(r#"crate "{crate_name}" not found"#))?
        .targets
        .iter()
        .find(|target| target.is_lib())
        .ok_or_else(|| anyhow!(r#"crate "{crate_name}" has no lib target"#))?
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
