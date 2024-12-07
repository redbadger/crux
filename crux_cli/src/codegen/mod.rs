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
use node::{CrateNode, ItemNode, SummaryNode};
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

    let manifest_paths: BTreeMap<&str, &str> = package_graph
        .packages()
        .map(|package| (package.name(), package.manifest_path().as_str()))
        .collect();

    println!("{:#?}", manifest_paths);

    let registry = run(crate_);

    println!("{:#?}", registry);

    Ok(())
}

fn run(crate_: Crate) -> Registry {
    let mut filter = Filter::default();
    filter.summary = crate_
        .paths
        .iter()
        .map(|(id, summary)| {
            (SummaryNode {
                id: *id,
                summary: summary.clone(),
            },)
        })
        .collect::<Vec<_>>();
    filter.item = crate_
        .index
        .values()
        .map(|item| (ItemNode(item.clone()),))
        .collect::<Vec<_>>();
    filter.ext_crate = crate_
        .external_crates
        .iter()
        .map(|(id, crate_)| {
            (CrateNode {
                id: *id,
                crate_: crate_.clone(),
            },)
        })
        .collect::<Vec<_>>();
    filter.run();

    println!("{:#?}", filter.continue_with);

    let mut formatter = Formatter::default();
    formatter.edge = filter.edge;
    formatter.run();
    formatter.container.into_iter().collect()
}

#[cfg(test)]
mod tests;
