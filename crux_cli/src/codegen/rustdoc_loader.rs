use anyhow::{Context, Result};
use rustdoc_types::Crate;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Loader for rustdoc JSON files with caching
pub struct RustdocLoader {
    /// Cache of loaded rustdoc files
    cache: HashMap<String, Crate>,
    /// Target directory containing rustdoc JSON files
    target_dir: PathBuf,
}

impl RustdocLoader {
    pub fn new(target_dir: PathBuf) -> Self {
        Self {
            cache: HashMap::new(),
            target_dir,
        }
    }

    /// Load rustdoc for a crate, using cache if available
    pub fn load_crate(&mut self, crate_name: &str) -> Result<&Crate> {
        if !self.cache.contains_key(crate_name) {
            let crate_data = self.load_rustdoc_json(crate_name)?;
            self.cache.insert(crate_name.to_string(), crate_data);
        }
        Ok(self.cache.get(crate_name).unwrap())
    }

    /// Load rustdoc JSON from disk or generate it if needed
    fn load_rustdoc_json(&self, crate_name: &str) -> Result<Crate> {
        let json_path = self
            .target_dir
            .join("doc")
            .join(format!("{crate_name}.json"));

        // Check if rustdoc exists, if not generate it
        if !json_path.exists() {
            log::info!("Generating rustdoc for crate: {crate_name}");
            self.generate_rustdoc(crate_name)?;
        }

        log::debug!("Loading rustdoc from: {}", json_path.display());
        let json_str = std::fs::read_to_string(&json_path)
            .context(format!("Failed to read rustdoc for {crate_name}"))?;

        serde_json::from_str(&json_str)
            .context(format!("Failed to parse rustdoc for {crate_name}"))
    }

    /// Generate rustdoc JSON for a crate
    fn generate_rustdoc(&self, crate_name: &str) -> Result<()> {
        let status = Command::new("cargo")
            .env("RUSTDOCFLAGS", "-Z unstable-options --output-format json")
            .args([
                "+nightly",
                "doc",
                "--no-deps",
                "--document-private-items",
                "--package",
                crate_name,
                "--target-dir",
                self.target_dir.to_str().unwrap(),
            ])
            .status()
            .context(format!("Failed to run cargo doc for {crate_name}"))?;

        if !status.success() {
            anyhow::bail!("Failed to generate rustdoc for {}", crate_name);
        }

        Ok(())
    }

    /// Get the target directory, trying to detect it if not specified
    pub fn get_target_dir() -> Result<PathBuf> {
        // First try CARGO_TARGET_DIR env variable
        if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
            return Ok(PathBuf::from(target_dir));
        }

        // Try to find target directory using cargo metadata
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1"])
            .output()
            .context("Failed to run cargo metadata")?;

        let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
            .context("Failed to parse cargo metadata")?;

        if let Some(target_dir) = metadata["target_directory"].as_str() {
            return Ok(PathBuf::from(target_dir));
        }

        // Fallback to ./target
        Ok(PathBuf::from("./target"))
    }

    /// Check if a crate is already loaded in cache
    pub fn is_cached(&self, crate_name: &str) -> bool {
        self.cache.contains_key(crate_name)
    }

    /// Get a cached crate without loading
    pub fn get_cached(&self, crate_name: &str) -> Option<&Crate> {
        self.cache.get(crate_name)
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}