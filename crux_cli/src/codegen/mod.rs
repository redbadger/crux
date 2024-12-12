mod filter;
mod formatter;
mod indexed;
mod node;
mod serde;
mod serde_generate;

use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf, process::Command};

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

    let registry = run(lib.name(), true, |name| load_crate(&name, &manifest_paths))?;

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

fn load_crate(name: &str, manifest_paths: &BTreeMap<&str, &str>) -> Result<Crate> {
    // TODO: ensure that the user has installed the core rustdoc JSON files
    // e.g. `rustup component add --toolchain nightly rust-docs-json`

    let json_path = if let "core" | "alloc" | "std" = name {
        rustdoc_json_path()?.join(format!("{name}.json"))
    } else {
        let manifest_path = manifest_paths
            .get(name)
            .ok_or_else(|| anyhow!("unknown crate {}", name))?;
        rustdoc_json::Builder::default()
            .toolchain("nightly")
            .document_private_items(true)
            .manifest_path(manifest_path)
            .build()?
    };
    println!("loading {} from {}", name, json_path.to_string_lossy());

    let buf = &mut Vec::new();
    File::open(json_path)?.read_to_end(buf)?;
    let crate_ = serde_json::from_slice(buf)?;

    Ok(crate_)
}

fn rustdoc_json_path() -> Result<PathBuf> {
    let output = Command::new("rustup")
        .arg("which")
        .args(["--toolchain", "nightly"])
        .arg("rustc")
        .output()?;
    let rustc_path = std::str::from_utf8(&output.stdout)?.trim();
    let json_path = PathBuf::from(rustc_path)
        .parent()
        .ok_or_else(|| anyhow!("could not get parent of {}", rustc_path))?
        .parent()
        .ok_or_else(|| anyhow!("could not get grandparent of {}", rustc_path))?
        .join("share/doc/rust/json");

    Ok(json_path)
}

#[cfg(test)]
mod tests;
