use anyhow::Result;
use rustdoc_types::{Crate, Id, Item, ItemEnum, Visibility};
use std::collections::HashSet;

use super::registry::{AccessPath, ReExport, TypeEntry, TypeRegistry};
use super::rustdoc_loader::RustdocLoader;

/// Type resolver that scans crates and builds a type registry
pub struct TypeResolver<'a> {
    loader: &'a mut RustdocLoader,
    registry: TypeRegistry,
    target_crate: String,
}

impl<'a> TypeResolver<'a> {
    pub fn new(loader: &'a mut RustdocLoader, target_crate: String) -> Self {
        Self {
            loader,
            registry: TypeRegistry::new(),
            target_crate,
        }
    }

    /// Scan specified crates and build type registry with auto-discovery
    pub fn scan_crates(&mut self, initial_crate_names: &[String]) -> Result<()> {
        log::info!("Starting scan with {} initial crates", initial_crate_names.len());

        // Get workspace members for filtering
        let workspace_members = match super::workspace::WorkspaceInfo::discover() {
            Ok(ws) => ws.member_names().into_iter().collect::<HashSet<String>>(),
            Err(_) => {
                log::warn!("Failed to discover workspace members, will try to load all discovered crates");
                HashSet::new()
            }
        };

        let mut crates_to_scan: Vec<String> = initial_crate_names.to_vec();
        let mut scanned_crates = std::collections::HashSet::new();
        let mut discovered_crates = std::collections::HashSet::new();

        // Keep scanning until we've processed all discovered crates
        while let Some(crate_name) = crates_to_scan.pop() {
            if scanned_crates.contains(&crate_name) {
                continue;
            }

            log::debug!("Loading crate: {crate_name}");
            match self.loader.load_crate(&crate_name) {
                Ok(_) => {
                    scanned_crates.insert(crate_name.clone());
                }
                Err(e) => {
                    log::warn!("Failed to load crate '{}': {}. Skipping.", crate_name, e);
                    continue;
                }
            }

            // Scan for re-exports to discover new crates
            if let Some(crate_data) = self.loader.get_cached(&crate_name) {
                for item in crate_data.index.values() {
                    if let ItemEnum::Use(use_item) = &item.inner {
                        if matches!(item.visibility, Visibility::Public) {
                            let source_parts: Vec<&str> = use_item.source.split("::").collect();
                            if let Some(source_crate) = source_parts.first() {
                                let source_crate = (*source_crate).to_string();
                                // Only consider workspace members unless no workspace info available
                                let should_scan = workspace_members.is_empty() || workspace_members.contains(&source_crate);
                                
                                if should_scan && !scanned_crates.contains(&source_crate) && !discovered_crates.contains(&source_crate) {
                                    log::debug!("Discovered dependency: {source_crate} (from {crate_name})");
                                    discovered_crates.insert(source_crate.clone());
                                    crates_to_scan.push(source_crate);
                                } else if !should_scan {
                                    log::trace!("Skipping non-workspace crate: {source_crate}");
                                }
                            }
                        }
                    }
                }
            }
        }

        log::info!("Auto-discovered {} additional crates", discovered_crates.len());

        // Now process all loaded crates
        let all_crates: Vec<String> = scanned_crates.into_iter().collect();
        
        // First pass: Collect direct types
        for crate_name in &all_crates {
            if let Some(crate_data) = self.loader.get_cached(crate_name) {
                log::debug!("Scanning types in crate: {crate_name}");
                let crate_data = crate_data.clone();
                self.collect_direct_types(crate_name, &crate_data);
            }
        }

        // Second pass: Resolve re-exports
        for crate_name in &all_crates {
            if let Some(crate_data) = self.loader.get_cached(crate_name) {
                log::debug!("Resolving re-exports for crate: {crate_name}");
                let crate_data = crate_data.clone();
                self.resolve_reexports(crate_name, &crate_data);
            }
        }

        log::info!(
            "Type resolution complete: {} types found across {} crates",
            self.registry.types.len(),
            all_crates.len()
        );
        Ok(())
    }

    /// Collect directly defined types from a crate
    fn collect_direct_types(&mut self, crate_name: &str, crate_data: &Crate) {
        for (id, item) in &crate_data.index {
            match &item.inner {
                ItemEnum::Struct(_)
                | ItemEnum::Enum(_)
                | ItemEnum::Union(_)
                | ItemEnum::Trait(_)
                | ItemEnum::TypeAlias(_) => {
                    // Only collect public types or types from the target crate
                    if matches!(item.visibility, Visibility::Public)
                        || crate_name == self.target_crate
                    {
                        self.registry.add_type(
                            *id,
                            TypeEntry {
                                item: item.clone(),
                                origin_crate: crate_name.to_string(),
                                access_path: AccessPath::Direct,
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Resolve re-exports in a crate
    fn resolve_reexports(&mut self, crate_name: &str, crate_data: &Crate) {
        for item in crate_data.index.values() {
            if let ItemEnum::Use(use_item) = &item.inner {
                // Only process public re-exports
                if !matches!(item.visibility, Visibility::Public) {
                    continue;
                }

                // Parse the use source
                let source_parts: Vec<&str> = use_item.source.split("::").collect();
                if let Some(source_crate) = source_parts.first() {
                    // Check if this is a crate we're scanning
                    if self.loader.is_cached(source_crate) {
                        let reexport = ReExport {
                            from_crate: crate_name.to_string(),
                            pub_name: use_item.name.clone(),
                            source_path: use_item.source.clone(),
                            source_crate: (*source_crate).to_string(),
                            visibility: item.visibility.clone(),
                        };

                        self.registry.add_reexport(use_item.source.clone(), reexport);

                        // Try to find the actual type in the source crate
                        if let Some(source_crate_data) = self.loader.get_cached(source_crate) {
                            log::debug!("Looking for type: {} in crate: {}", use_item.source, source_crate);
                            if let Some(type_entry) =
                                Self::find_type_by_path(source_crate_data, &use_item.source)
                            {
                                log::debug!("Found re-exported type: {} -> {}", use_item.source, use_item.name);
                                // Add the type with re-export access path
                                let id = type_entry.0;
                                let mut entry = type_entry.1.clone();
                                entry.access_path = AccessPath::ReExported {
                                    export_path: format!("{}::{}", crate_name, use_item.name),
                                    source_path: use_item.source.clone(),
                                };
                                self.registry.add_type(id, entry);
                            } else {
                                log::warn!("Could not find type: {} in crate: {}", use_item.source, source_crate);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Find a type by its import path in a crate
    fn find_type_by_path(crate_data: &Crate, import_path: &str) -> Option<(Id, TypeEntry)> {
        // Parse the import path
        let path_parts: Vec<&str> = import_path.split("::").collect();
        if path_parts.is_empty() {
            return None;
        }

        let crate_name = path_parts[0];
        let type_path = &path_parts[1..];
        let type_name = type_path.last()?;
        
        log::trace!("Searching for type '{}' in path {:?}", type_name, type_path);

        // Search through the crate's index
        for (id, item) in &crate_data.index {
            // For now, just match on the type name since we don't have full path support
            if let Some(name) = &item.name {
                if name == type_name {
                    log::trace!("Found matching item: {} (id: {:?})", name, id);
                    return Some((
                        *id,
                        TypeEntry {
                            item: item.clone(),
                            origin_crate: crate_name.to_string(),
                            access_path: AccessPath::Direct,
                        },
                    ));
                }
            }
        }

        log::trace!("Type '{}' not found in crate", type_name);
        None
    }

    /// Get the path of an item within its crate
    fn get_item_path(item: &Item, _crate_data: &Crate) -> Option<Vec<String>> {
        // Build path from item name and parent modules
        let mut path = vec![];

        if let Some(name) = &item.name {
            path.push(name.clone());
        } else {
            return None;
        }

        // TODO: Walk up parent modules to build full path
        // This would require following item.parent links in the crate data

        Some(path)
    }

    /// Check if an item path matches the expected path
    fn path_matches(item_path: &[String], expected_path: &[&str]) -> bool {
        if item_path.len() != expected_path.len() {
            return false;
        }

        item_path
            .iter()
            .zip(expected_path.iter())
            .all(|(a, b)| a == b)
    }

    /// Get all types that should be included in codegen for the target crate
    pub fn get_visible_types(&self) -> Vec<&TypeEntry> {
        self.registry.get_visible_types(&self.target_crate)
    }

    /// Get the type registry (consumes the resolver)
    pub fn into_registry(self) -> TypeRegistry {
        self.registry
    }
}