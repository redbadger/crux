mod format;
mod logic;

use std::fs::File;

use anyhow::{bail, Result};
use guppy::{graph::PackageGraph, MetadataCommand};
use logic::Node;
use rustdoc_types::Crate;

use crate::args::CodegenArgs;

pub fn codegen(args: &CodegenArgs) -> Result<()> {
    let mut cmd = MetadataCommand::new();
    let package_graph = PackageGraph::from_command(&mut cmd)?;

    let Ok(lib) = package_graph.workspace().member_by_path(&args.lib) else {
        bail!("Could not find workspace package with path {}", args.lib)
    };

    let json_path = rustdoc_json::Builder::default()
        .toolchain("nightly")
        .document_private_items(true)
        .manifest_path(lib.manifest_path())
        .build()?;

    let file = File::open(json_path)?;
    let crate_: Crate = serde_json::from_reader(file)?;

    let nodes = crate_
        .index
        .values()
        .flat_map(|item| {
            if item.attrs.contains(&"#[serde(skip)]".to_string()) {
                None
            } else {
                Some((Node {
                    id: item.id,
                    item: Some(item.clone()),
                    summary: crate_.paths.get(&item.id).cloned(),
                },))
            }
        })
        .collect::<Vec<_>>();

    let registry = logic::run(nodes);
    println!("{:#?}", registry);

    Ok(())
}
