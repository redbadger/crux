use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use std::collections::HashMap;
use std::path::PathBuf;

/// Information about the current workspace
#[derive(Debug)]
pub struct WorkspaceInfo {
    /// All packages in the workspace
    pub members: Vec<PackageInfo>,
    /// Dependency relationships between workspace members
    pub dependencies: HashMap<String, Vec<String>>,
    /// The workspace root directory
    pub root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
}

impl WorkspaceInfo {
    /// Discover workspace information using cargo metadata
    pub fn discover() -> Result<Self> {
        let metadata = MetadataCommand::new()
            .exec()
            .context("Failed to get cargo metadata")?;

        let members = Self::extract_workspace_members(&metadata);
        let dependencies = Self::build_dependency_graph(&metadata);
        let root = metadata.workspace_root.into();

        Ok(Self {
            members,
            dependencies,
            root,
        })
    }

    /// Extract workspace member information
    fn extract_workspace_members(metadata: &Metadata) -> Vec<PackageInfo> {
        metadata
            .workspace_members
            .iter()
            .filter_map(|id| {
                metadata.packages.iter().find(|p| &p.id == id).map(|p| {
                    PackageInfo {
                        name: p.name.clone(),
                        version: p.version.to_string(),
                        manifest_path: p.manifest_path.clone().into(),
                    }
                })
            })
            .collect()
    }

    /// Build dependency graph for workspace members only
    fn build_dependency_graph(metadata: &Metadata) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();

        // Get set of workspace member names for quick lookup
        let workspace_member_names: std::collections::HashSet<_> = metadata
            .workspace_members
            .iter()
            .filter_map(|id| {
                metadata
                    .packages
                    .iter()
                    .find(|p| &p.id == id)
                    .map(|p| p.name.clone())
            })
            .collect();

        // Build graph for workspace members
        for package in &metadata.packages {
            if !workspace_member_names.contains(&package.name) {
                continue;
            }

            let deps: Vec<String> = package
                .dependencies
                .iter()
                .filter(|d| workspace_member_names.contains(&d.name))
                .map(|d| d.name.clone())
                .collect();

            graph.insert(package.name.clone(), deps);
        }

        graph
    }

    /// Get all workspace member names
    pub fn member_names(&self) -> Vec<String> {
        self.members.iter().map(|m| m.name.clone()).collect()
    }

    /// Get dependencies for a specific crate
    pub fn get_dependencies(&self, crate_name: &str) -> Option<&Vec<String>> {
        self.dependencies.get(crate_name)
    }

    /// Check if a crate is a workspace member
    pub fn is_workspace_member(&self, crate_name: &str) -> bool {
        self.members.iter().any(|m| m.name == crate_name)
    }

    /// Get topological order of crates (dependencies first)
    pub fn topological_order(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        for member in &self.members {
            if !visited.contains(&member.name) {
                self.dfs_visit(
                    &member.name,
                    &mut visited,
                    &mut visiting,
                    &mut result,
                )?;
            }
        }

        Ok(result)
    }

    /// Depth-first search for topological ordering
    fn dfs_visit(
        &self,
        crate_name: &str,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<()> {
        if visiting.contains(crate_name) {
            anyhow::bail!("Circular dependency detected involving {}", crate_name);
        }

        if visited.contains(crate_name) {
            return Ok(());
        }

        visiting.insert(crate_name.to_string());

        if let Some(deps) = self.dependencies.get(crate_name) {
            for dep in deps {
                self.dfs_visit(dep, visited, visiting, result)?;
            }
        }

        visiting.remove(crate_name);
        visited.insert(crate_name.to_string());
        result.push(crate_name.to_string());

        Ok(())
    }
}