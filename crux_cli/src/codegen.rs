use anyhow::{bail, Result};
use rustdoc_types::{Crate, Impl, ItemEnum, Path};
use std::{
    fs::File,
    io::{stdout, IsTerminal},
};
use tokio::{process::Command, task::spawn_blocking};

use crate::{args::CodegenArgs, command_runner, graph};

pub async fn codegen(args: &CodegenArgs) -> Result<()> {
    let graph = graph::compute_package_graph()?;

    let Ok(lib) = graph.workspace().member_by_path(&args.lib) else {
        bail!("Could not find workspace package with path {}", args.lib)
    };

    let mut cmd = Command::new("cargo");
    cmd.env("RUSTC_BOOTSTRAP", "1")
        .env(
            "RUSTDOCFLAGS",
            "-Z unstable-options --output-format=json --cap-lints=allow",
        )
        .arg("doc")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(lib.manifest_path())
        .arg("--lib");
    if stdout().is_terminal() {
        cmd.arg("--color=always");
    }

    command_runner::run(&mut cmd).await?;

    let target_directory = graph.workspace().target_directory().as_std_path();
    let json_path = target_directory
        .join("doc")
        .join(format!("{}.json", lib.name().replace("-", "_")));

    let rustdoc: Crate = spawn_blocking(move || {
        let file = File::open(&json_path)?;
        let crate_: Crate = serde_json::from_reader(file)?;
        Ok::<rustdoc_types::Crate, anyhow::Error>(crate_)
    })
    .await??;

    if let Some(name) = rustdoc.index.iter().find_map(|(_k, v)| {
        if let ItemEnum::Impl(Impl {
            trait_: Some(rustdoc_types::Path {
                name: trait_name, ..
            }),
            for_: rustdoc_types::Type::ResolvedPath(Path { name, .. }),
            ..
        }) = &v.inner
        {
            (trait_name == &"App".to_string()).then(|| name)
        } else {
            None
        }
    }) {
        println!(
            "The struct that implements crux_core::App is called {:?}",
            name
        );
    }

    Ok(())
}
