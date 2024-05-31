mod graph;
mod parser;
mod rust_types;

use anyhow::{bail, Result};
use rustdoc_types::Crate;
use std::{
    fs::File,
    io::{stdout, IsTerminal},
};
use tokio::{process::Command, task::spawn_blocking};

use crate::{args::CodegenArgs, command_runner};

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
        .join(format!("{}.json", lib.name().replace('-', "_")));

    let crate_: Crate = spawn_blocking(move || -> Result<Crate> {
        let file = File::open(json_path)?;
        let crate_ = serde_json::from_reader(file)?;
        Ok(crate_)
    })
    .await??;

    let data = parser::parse(crate_)?;
    println!("\n\ndata: {data:?}");

    Ok(())
}
