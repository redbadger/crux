mod filter;
mod formatter;
mod indexed;
mod node;
mod serde_generate;

use std::{collections::BTreeMap, fs::File};

use anyhow::{anyhow, bail, Result};
use guppy::{graph::PackageGraph, MetadataCommand};
use iter_tools::Itertools as _;
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

    let manifest_paths: BTreeMap<&str, &str> = package_graph
        .packages()
        .map(|package| (package.name(), package.manifest_path().as_str()))
        .collect();

    let Ok(lib) = package_graph.workspace().member_by_path(&args.lib) else {
        bail!("Could not find workspace package with path {}", args.lib)
    };

    let registry = run(lib.name(), false, |name| load_crate(&name, &manifest_paths))?;

    println!("{:#?}", registry);

    Ok(())
}

fn run(
    crate_name: &str,
    should_recurse: bool,
    load: impl Fn(String) -> Result<Crate>,
) -> Result<Registry> {
    let shared_lib = load(crate_name.to_string())?;
    let (mut filtered, _continue_with) = filter(shared_lib, vec![]);
    let mut summaries = _continue_with
        .into_iter()
        .into_group_map()
        .into_iter()
        .collect::<Vec<_>>();

    println!("summaries {:#?}", summaries);

    if should_recurse {
        while !summaries.is_empty() {
            let (crate_, items) = summaries.pop().unwrap();
            println!("loading {}", crate_.crate_.name);
            let dep = crate_.crate_.name.clone();
            let crate_ = load(dep)?;
            let (more_filtered, more_continue_with) = filter(crate_, items);
            filtered.extend(more_filtered);
            let more_summaries = more_continue_with
                .into_iter()
                .into_group_map()
                .into_iter()
                .collect::<Vec<_>>();
            println!("summaries {:#?}", more_summaries);
            summaries.extend(more_summaries);
        }
    }
    Ok(format(filtered))
}

fn filter(
    crate_: Crate,
    summaries: Vec<SummaryNode>,
) -> (Vec<(ItemNode, ItemNode)>, Vec<(CrateNode, SummaryNode)>) {
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
    filter.start_with = summaries.into_iter().map(|summary| (summary,)).collect();
    filter.run();

    (filter.edge, filter.continue_with)
}

fn format(edges: Vec<(ItemNode, ItemNode)>) -> Registry {
    let mut formatter = Formatter::default();
    formatter.edge = edges;
    formatter.run();

    formatter.container.into_iter().collect()
}

fn load_crate(name: &str, manifest_paths: &BTreeMap<&str, &str>) -> Result<Crate, anyhow::Error> {
    let manifest_path = manifest_paths
        .get(name)
        .ok_or_else(|| anyhow!("cannot get manifest for crate {name}"))?;
    let json_path = rustdoc_json::Builder::default()
        .toolchain("nightly")
        .document_private_items(true)
        .manifest_path(manifest_path)
        .build()?;
    let file = File::open(json_path)?;
    let crate_ = serde_json::from_reader(file)?;

    Ok(crate_)
}

#[cfg(test)]
mod tests;
