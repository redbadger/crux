use rustdoc_types::{Id, Item, Visibility};
use std::collections::HashMap;

/// Represents a type definition with its origin
#[derive(Debug, Clone)]
pub struct TypeEntry {
    /// The actual type information from rustdoc
    pub item: Item,
    /// Which crate this type was originally defined in
    pub origin_crate: String,
    /// How this type is accessible (direct or re-exported)
    pub access_path: AccessPath,
}

#[derive(Debug, Clone)]
pub enum AccessPath {
    /// Type is directly defined in the target crate
    Direct,
    /// Type is re-exported from another crate
    ReExported {
        /// The re-export path in the target crate
        export_path: String,
        /// The original path in the source crate
        source_path: String,
    },
}

/// Registry of all types found across scanned crates
#[derive(Debug)]
pub struct TypeRegistry {
    /// All discovered types by their ID
    pub types: HashMap<Id, TypeEntry>,
    /// Re-export relationships
    pub reexports: HashMap<String, Vec<ReExport>>,
    /// Crate dependency graph
    pub crate_graph: CrateGraph,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            reexports: HashMap::new(),
            crate_graph: CrateGraph::new(),
        }
    }

    /// Add a type to the registry
    pub fn add_type(&mut self, id: Id, entry: TypeEntry) {
        self.types.insert(id, entry);
    }

    /// Add a re-export relationship
    pub fn add_reexport(&mut self, source_path: String, reexport: ReExport) {
        self.reexports
            .entry(source_path)
            .or_default()
            .push(reexport);
    }

    /// Get all types visible from a specific crate
    pub fn get_visible_types(&self, target_crate: &str) -> Vec<&TypeEntry> {
        self.types
            .values()
            .filter(|entry| self.is_visible_from(entry, target_crate))
            .collect()
    }

    /// Check if a type is visible from the target crate
    fn is_visible_from(&self, entry: &TypeEntry, target_crate: &str) -> bool {
        // Include if directly defined in target crate
        if entry.origin_crate == target_crate {
            return true;
        }

        // Include if re-exported by target crate
        if let AccessPath::ReExported { export_path, .. } = &entry.access_path {
            // Check if any re-export originates from the target crate
            if let Some(reexports) = self.reexports.get(export_path) {
                return reexports.iter().any(|r| r.from_crate == target_crate);
            }
        }

        false
    }
}

#[derive(Debug, Clone)]
pub struct ReExport {
    /// Crate doing the re-exporting
    pub from_crate: String,
    /// The public name of the re-export
    pub pub_name: String,
    /// The source path being re-exported
    pub source_path: String,
    /// The source crate
    pub source_crate: String,
    /// Visibility of the re-export
    pub visibility: Visibility,
}

/// Simple crate dependency graph
#[derive(Debug)]
pub struct CrateGraph {
    /// Dependencies for each crate
    pub dependencies: HashMap<String, Vec<String>>,
}

impl CrateGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    /// Add a dependency relationship
    pub fn add_dependency(&mut self, crate_name: String, depends_on: String) {
        self.dependencies
            .entry(crate_name)
            .or_default()
            .push(depends_on);
    }

    /// Get all dependencies for a crate
    pub fn get_dependencies(&self, crate_name: &str) -> Option<&Vec<String>> {
        self.dependencies.get(crate_name)
    }
}