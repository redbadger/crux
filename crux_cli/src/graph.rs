use anyhow::{anyhow, Result};
use guppy::{graph::PackageGraph, MetadataCommand};

pub(crate) fn compute_package_graph() -> Result<PackageGraph> {
    let mut cmd = MetadataCommand::new();
    let package_graph = PackageGraph::from_command(&mut cmd);
    package_graph.map_err(|e| anyhow!(e))
}
