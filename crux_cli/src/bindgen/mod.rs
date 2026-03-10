use anyhow::{Context as _, Result, anyhow};
use camino::Utf8PathBuf;
use cargo_metadata::{Metadata, MetadataCommand};
use uniffi_bindgen::bindings::{GenerateOptions, TargetLanguage, generate};

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

    if let Some(out_dir) = &args.languages.kotlin {
        generate(GenerateOptions {
            languages: vec![TargetLanguage::Kotlin],
            source: library_path.clone(),
            out_dir: Utf8PathBuf::from_path_buf(out_dir.clone())
                .map_err(|p| anyhow!("path {} has non-unicode characters", p.display()))?,
            ..Default::default()
        })
        .context("generating Kotlin bindings")?;
    }

    if let Some(out_dir) = &args.languages.swift {
        generate(GenerateOptions {
            languages: vec![TargetLanguage::Swift],
            source: library_path,
            out_dir: Utf8PathBuf::from_path_buf(out_dir.clone())
                .map_err(|p| anyhow!("path {} has non-unicode characters", p.display()))?,
            ..Default::default()
        })
        .context("generating Swift bindings")?;
    }

    Ok(())
}

fn find_library_path(metadata: &Metadata, crate_name: &String) -> Result<Utf8PathBuf> {
    let library_name = metadata
        .workspace_packages()
        .iter()
        .find(|package| &package.name == crate_name)
        .ok_or_else(|| anyhow!(r#"crate "{crate_name}" not found"#))?
        .targets
        .iter()
        .find(|target| target.is_lib())
        .ok_or_else(|| anyhow!(r#"crate "{crate_name}" has no lib target"#))?
        .name
        .clone();
    let target_dir = &metadata.target_directory;
    let library_path = &target_dir.join(format!("debug/lib{library_name}"));
    // Prefer shared libraries (dylib/so/dll) over static (rlib/a) because UniFFI
    // library mode needs cross-crate metadata embedded in the linked shared library.
    let library_path = ["dylib", "so", "dll", "rlib", "a"]
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
