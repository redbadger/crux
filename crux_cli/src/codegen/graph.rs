use anyhow::Result;
use guppy::{graph::PackageGraph, MetadataCommand};

pub(crate) fn compute_package_graph() -> Result<PackageGraph> {
    let mut cmd = MetadataCommand::new();
    let package_graph = PackageGraph::from_command(&mut cmd)?;
    Ok(package_graph)
}
