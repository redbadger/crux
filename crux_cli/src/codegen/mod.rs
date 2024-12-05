mod filter;
mod formatter;
mod indexed;
mod node;
mod serde_generate;

use std::{collections::BTreeMap, fs::File};

use anyhow::{bail, Result};
use guppy::{graph::PackageGraph, MetadataCommand};
use rustdoc_types::Crate;

use crate::args::CodegenArgs;
use filter::Filter;
use formatter::Formatter;
use node::Node;
use serde_generate::format::ContainerFormat;

pub type Registry = BTreeMap<String, ContainerFormat>;

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

    let nodes = parse(crate_);

    let registry = run(nodes);

    println!("{:#?}", registry);

    Ok(())
}

fn run(nodes: Vec<(Node,)>) -> Registry {
    let mut filter = Filter::default();
    filter.node = nodes;
    filter.run();

    let mut formatter = Formatter::default();
    formatter.edge = filter.edge;
    formatter.run();
    formatter.container.into_iter().collect()
}

fn parse(crate_: Crate) -> Vec<(Node,)> {
    crate_
        .index
        .values()
        .map(|item| {
            (Node {
                id: item.id,
                item: Some(item.clone()),
                summary: crate_.paths.get(&item.id).cloned(),
            },)
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests;
