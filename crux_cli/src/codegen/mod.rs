mod filter;
mod formatter;
pub mod generate;
mod indexed;
mod item;
mod node;
mod serde;
mod serde_generate;

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::Read,
    process::Command,
};

use anyhow::{bail, Context, Result};
use guppy::{graph::PackageGraph, MetadataCommand};
use log::debug;
use rustdoc_types::Crate;

use crate::args::CodegenArgs;
use filter::Filter;
use formatter::Formatter;
use node::ItemNode;
use serde_generate::format::ContainerFormat;

pub type Registry = BTreeMap<String, ContainerFormat>;

pub fn codegen(args: &CodegenArgs) -> Result<()> {
    let mut cmd = MetadataCommand::new();
    let package_graph = PackageGraph::from_command(&mut cmd)?;

    let manifest_paths: BTreeMap<&str, &str> = package_graph
        .packages()
        .map(|package| (package.name(), package.manifest_path().as_str()))
        .collect();

    let Ok(lib) = package_graph.workspace().member_by_name(&args.crate_name) else {
        bail!(
            "Could not find workspace package with name {}",
            args.crate_name
        )
    };

    let lib_name = lib.name();
    let registry = run(lib_name, |name| {
        log::debug!("Filter requesting crate: {name}");
        let crate_data = load_crate(name, &manifest_paths)?;
        
        // Log general information about loaded crates for debugging
        let type_count = crate_data.index.len();
        let type_names: Vec<&str> = crate_data.index.values()
            .filter_map(|item| item.name.as_deref())
            .take(10) // Show first 10 types as sample
            .collect();
        log::debug!("Loaded crate '{name}' with {type_count} types (sample: {type_names:?})");
        
        Ok(crate_data)
    })?;
    
    log::info!("Generated {} types: {:?}", registry.len(), registry.keys().collect::<Vec<_>>());

    // switch from vendored types to `serde-reflection` types
    let registry: serde_reflection::Registry =
        serde_json::from_slice(&serde_json::to_vec(&registry)?)?;

    fs::create_dir_all(&args.out_dir)?;

    if let Some(java_package) = &args.generate.java {
        generate::java(&registry, java_package, args.out_dir.join("java"))
            .context("Generating types for Java")?;
    }

    if let Some(swift_package) = &args.generate.swift {
        generate::swift(&registry, swift_package, args.out_dir.join("swift"))
            .context("Generating types for Swift")?;
    }

    if let Some(typescript_package) = &args.generate.typescript {
        // Get version from workspace metadata
        let version = {
            let mut cmd = MetadataCommand::new();
            let package_graph = PackageGraph::from_command(&mut cmd)?;
            package_graph
                .workspace()
                .member_by_name(&args.crate_name)
                .map_or_else(|_| "0.1.0".to_string(), |pkg| pkg.version().to_string())
        };
        
        generate::typescript(
            &registry,
            typescript_package,
            &version,
            args.out_dir.join("typescript"),
        )
        .context("Generating types for TypeScript")?;
    }

    Ok(())
}

fn run<F>(crate_name: &str, load: F) -> Result<Registry>
where
    F: Fn(&str) -> Result<Crate>,
{
    let mut previous: HashMap<String, Crate> = HashMap::new();

    let shared_lib = load(crate_name)?;

    let mut filter = Filter::default();
    filter.process(crate_name, &shared_lib);

    previous.insert(crate_name.to_string(), shared_lib);

    let mut next: Vec<String> = filter.get_crates();

    while let Some(crate_name) = next.pop() {
        if previous.contains_key(&crate_name) {
            continue;
        }
        
        // Skip built-in Rust crates that don't have manifests
        if matches!(crate_name.as_str(), "std" | "core" | "alloc" | "proc_macro" | "test") {
            continue;
        }
        
        // Skip non-workspace crates
        let workspace_members = get_workspace_members()?;
        if !workspace_members.contains(&crate_name) {
            debug!("Skipping non-workspace crate: {crate_name}");
            continue;
        }
        
        let crate_ = load(&crate_name)?;

        filter.process(&crate_name, &crate_);

        next = filter.get_crates();
        previous.insert(crate_name, crate_);
    }

    Ok(format(filter.edge))
}

fn format(edges: Vec<(ItemNode, ItemNode)>) -> Registry {
    let mut formatter = Formatter::default();
    formatter.edge = edges;
    formatter.run();
    debug!("{}", formatter.scc_times_summary());

    formatter.container.into_iter().collect()
}

fn get_workspace_members() -> Result<Vec<String>> {
    let metadata = cargo_metadata::MetadataCommand::new().exec()?;
    let members = metadata.workspace_members
        .iter()
        .map(|id| id.repr.split('#').next().unwrap_or(&id.repr).split('/').next_back().unwrap_or(&id.repr).to_string())
        .collect();
    Ok(members)
}

fn load_crate(name: &str, manifest_paths: &BTreeMap<&str, &str>) -> Result<Crate> {
    // Check if we have a manifest path
    if let Some(manifest_path) = manifest_paths.get(name) {

    let status = Command::new("cargo")
        .env("RUSTC_BOOTSTRAP", "1")
        .env(
            "RUSTDOCFLAGS",
            "-Z unstable-options --output-format=json --cap-lints=allow",
        )
        .arg("doc")
        .arg("--no-deps")
        .arg("--lib")
        .args(["--manifest-path", manifest_path])
        .arg("--all-features")
        .arg("--document-private-items")
        .status()?;

    if !status.success() {
        bail!("failed to generate rustdoc json for {manifest_path} with error code {status}");
    }

    let mut metadata = cargo_metadata::MetadataCommand::new();
    metadata.manifest_path(manifest_path);
    let mut json_path = metadata.exec()?.target_directory;
    json_path.push("doc");
    json_path.push(name);
    json_path.set_extension("json");

    debug!("from {json_path}");

    let buf = &mut Vec::new();
    File::open(json_path)?.read_to_end(buf)?;
    let crate_ = serde_json::from_slice(buf).context(
        r"
There was a problem reading RustDoc JSON output — maybe there is
a format version incompatibility.
We currently require format version >=39, which means Rust >=1.86.
Please raise an issue at https://github.com/redbadger/crux/issue and
include the version of Rust that you are using. Thank you!",
    )?;

        Ok(crate_)
    } else {
        // No manifest path found, try workspace member
        debug!("No manifest path for {name}, trying as workspace member");
        
        // Use standard cargo doc command for workspace members
        let status = Command::new("cargo")
            .env("RUSTDOCFLAGS", "-Z unstable-options --output-format json")
            .args([
                "+nightly",
                "doc",
                "--no-deps",
                "--document-private-items",
                "--package",
                name,
            ])
            .status()?;

        if !status.success() {
            bail!("failed to generate rustdoc json for workspace crate {}", name);
        }

        // Get target directory from cargo metadata
        let metadata = cargo_metadata::MetadataCommand::new().exec()?;
        let mut json_path = metadata.target_directory.as_std_path().to_path_buf();
        json_path.push("doc");
        json_path.push(name);
        json_path.set_extension("json");

        debug!("from {}", json_path.display());

        let buf = &mut Vec::new();
        File::open(json_path)?.read_to_end(buf)?;
        let crate_ = serde_json::from_slice(buf).context(
            r"
There was a problem reading RustDoc JSON output — maybe there is
a format version incompatibility.
We currently require format version >=39, which means Rust >=1.86.
Please raise an issue at https://github.com/redbadger/crux/issue and
include the version of Rust that you are using. Thank you!",
        )?;

        Ok(crate_)
    }
}

pub fn collect<'a, N, T>(input: T) -> impl Iterator<Item = Vec<(&'a N,)>>
where
    N: 'a + Clone,
    T: Iterator<Item = (&'a N,)>,
{
    std::iter::once(input.collect::<Vec<_>>())
}


#[cfg(test)]
mod tests;
