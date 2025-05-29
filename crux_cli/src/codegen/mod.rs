mod filter;
mod formatter;
pub mod generate;
mod indexed;
mod item;
mod node;
mod serde;
mod serde_generate;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File},
    io::Read,
    process::Command,
};

use anyhow::{bail, Context, Result};
use guppy::{graph::PackageGraph, MetadataCommand};
use log::debug;
use rustdoc_types::Crate;

use crate::args::CodegenArgs;
use filter::{Filter, STD_CRATES};
use formatter::Formatter;
use node::ItemNode;
use serde_generate::format::ContainerFormat;

pub type Registry = BTreeMap<String, ContainerFormat>;

/// Get all workspace member crate names using `cargo_metadata`
fn get_all_workspace_members() -> Result<HashSet<String>> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .context("Failed to execute cargo metadata")?;
    
    let workspace_members: HashSet<_> = metadata.workspace_members.iter().collect();
    
    let member_names: HashSet<String> = metadata
        .packages
        .iter()
        .filter(|pkg| workspace_members.contains(&pkg.id))
        .map(|pkg| pkg.name.clone())
        .collect();
    
    Ok(member_names)
}

/// Find workspace crates that are actually referenced in the type system
fn find_referenced_workspace_crates(
    filter: &Filter,
    workspace_members: &HashSet<String>,
    exclude: &str,
) -> Vec<String> {
    let mut referenced = HashSet::new();
    
    // Check all external types to see which workspace crates are referenced
    for external_type in filter.get_external_types() {
        if let Some(crate_name) = external_type.actual_crate_name() {
            if workspace_members.contains(&crate_name) && crate_name != exclude {
                referenced.insert(crate_name);
            }
        }
    }
    
    // Only return crates that have library targets
    let Ok(metadata) = cargo_metadata::MetadataCommand::new().exec() else {
        return Vec::new();
    };
    
    referenced
        .into_iter()
        .filter(|name| has_library_target(name, &metadata))
        .collect()
}

/// Check if a crate should be skipped in the dependency loop
fn should_skip_crate(
    crate_name: &str,
    workspace_members: &HashSet<String>,
    previous: &HashMap<String, Crate>,
) -> bool {
    // Already processed
    if previous.contains_key(crate_name) {
        return true;
    }
    
    // Built-in Rust crates
    if STD_CRATES.contains(&crate_name) {
        return true;
    }
    
    // Workspace members are handled separately
    if workspace_members.contains(crate_name) {
        return true;
    }
    
    // Skip all external dependencies for now
    // TODO: The challenge is that we'd need the field information for these types to generate proper serialization code.
    // So we'd need a hybrid approach:
    // - Use synthetic types for the structure
    // - Or have crux crates provide pre-generated type definitions
    // - Or selectively load only the needed types from crux crates
    if !workspace_members.contains(crate_name) {
        return true;
    }
    
    false
}

/// Check if a crate has a library target
fn has_library_target(crate_name: &str, metadata: &cargo_metadata::Metadata) -> bool {
    metadata
        .packages
        .iter()
        .find(|pkg| pkg.name == crate_name)
        .is_some_and(|pkg| {
            pkg.targets.iter().any(|target| {
                target.is_lib()
                    || target.is_cdylib()
                    || target.is_dylib()
                    || target.is_rlib()
                    || target.is_staticlib()
            })
        })
}


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

    let registry = run(lib_name, |name| load_crate(name, &manifest_paths))?;

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
        generate::typescript(
            &registry,
            typescript_package,
            &lib.version().to_string(),
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
    
    // Phase 1: Process the main crate
    let shared_lib = load(crate_name)?;
    let mut filter = Filter::default();
    filter.process(crate_name, &shared_lib);
    filter.add_all_public_types_as_roots(crate_name);
    previous.insert(crate_name.to_string(), shared_lib);
    
    // Phase 2: Identify and load referenced workspace crates
    let workspace_members = get_all_workspace_members().unwrap_or_default();
    
    let referenced_workspace_crates = find_referenced_workspace_crates(
        &filter,
        &workspace_members,
        crate_name,
    );
    
    for workspace_crate in referenced_workspace_crates {
        if let Ok(crate_data) = load(&workspace_crate) {
            filter.process(&workspace_crate, &crate_data);
            filter.add_all_public_types_as_roots(&workspace_crate);
            previous.insert(workspace_crate, crate_data);
        }
    }
    
    // Phase 3: Process external dependencies (only crux_ crates)
    let mut next = filter.get_crates();
    while let Some(crate_name) = next.pop() {
        if should_skip_crate(&crate_name, &workspace_members, &previous) {
            continue;
        }
        
        let crate_ = load(&crate_name)?;
        filter.process(&crate_name, &crate_);
        next = filter.get_crates();
        previous.insert(crate_name, crate_);
    }

    // Phase 4: Handle remaining external types (non-workspace)
    // Only create synthetic types for truly external types (e.g., chrono::DateTime)
    let external_types: Vec<_> = filter.get_external_types()
        .into_iter()
        .filter(|t| !t.is_workspace_type(&workspace_members))
        .collect();
    
    if !external_types.is_empty() {
        filter.add_workspace_external_types(external_types);
    }

    Ok(format(filter.edge))
}

fn format(edges: Vec<(ItemNode, ItemNode)>) -> Registry {
    let mut formatter = Formatter::default();
    formatter.edge = edges;
    formatter.run();
    
    formatter.container.into_iter().collect()
}

fn load_crate(name: &str, manifest_paths: &BTreeMap<&str, &str>) -> Result<Crate> {
    let manifest_path = manifest_paths
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("unknown crate {}", name))?;

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
