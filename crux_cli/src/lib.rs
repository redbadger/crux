mod config;
mod manifest;
mod rustdoc_cmd;
mod util;

use anyhow::Result;
pub use config::GlobalConfig;
use manifest::Manifest;
use rustdoc_cmd::RustdocCommand;
use std::path::{Path, PathBuf};

pub fn produce_doc(manifest_path: impl AsRef<Path>, config: &mut GlobalConfig) -> Result<PathBuf> {
    let manifest = Manifest::parse(manifest_path.as_ref().to_path_buf())?;

    let name = crate::manifest::get_package_name(&manifest)?;
    let version = crate::manifest::get_package_version(&manifest)?;

    let rustdoc_cmd = RustdocCommand::new()
        .deps(false)
        .silence(!config.is_verbose());

    let rustdoc_path = rustdoc_cmd.dump(
        config,
        manifest_path.as_ref(),
        Some(&format!("{name}@{version}")),
        false,
    )?;

    Ok(rustdoc_path)
}
