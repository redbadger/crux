mod crate_wrapper;
mod graph;
mod intermediate_public_item;
mod item_processor;
mod nameable_item;
mod parser;
mod path_component;
mod public_item;
use rustdoc_types::{Crate, Id, Item};

/// The [`Crate`] type represents the deserialized form of the rustdoc JSON
/// input. This wrapper adds some helpers and state on top.
pub struct CrateWrapper<'c> {
    crate_: &'c Crate,

    /// Normally, an item referenced by [`Id`] is present in the rustdoc JSON.
    /// If [`Self::crate_.index`] is missing an [`Id`], then we add it here, to
    /// aid with debugging. It will typically be missing because of bugs (or
    /// borderline bug such as re-exports of foreign items like discussed in
    /// <https://github.com/rust-lang/rust/pull/99287#issuecomment-1186586518>)
    /// We do not report it to users by default, because they can't do anything
    /// about it. Missing IDs will be printed with `--verbose` however.
    missing_ids: Vec<&'c Id>,
}

impl<'c> CrateWrapper<'c> {
    pub fn new(crate_: &'c Crate) -> Self {
        Self {
            crate_,
            missing_ids: vec![],
        }
    }

    pub fn get_item(&mut self, id: &'c Id) -> Option<&'c Item> {
        self.crate_.index.get(id).or_else(|| {
            self.missing_ids.push(id);
            None
        })
    }

    pub fn missing_item_ids(&self) -> Vec<String> {
        self.missing_ids.iter().map(|m| m.0.clone()).collect()
    }
}
mod render;
mod rust_types;
mod tokens;

use anyhow::{bail, Result};
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

    let data = parser::parse(&crate_)?;
    println!("\n\ndata: {data:?}");

    Ok(())
}
