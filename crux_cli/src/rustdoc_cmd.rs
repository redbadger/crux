use crate::GlobalConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustdocCommand {
    deps: bool,
    silence: bool,
}

impl RustdocCommand {
    pub(crate) fn new() -> Self {
        Self {
            deps: false,
            silence: false,
        }
    }

    /// Include dependencies
    ///
    /// Reasons to have this disabled:
    /// - Faster API extraction
    /// - Less likely to hit bugs in rustdoc, like
    ///   - rust-lang/rust#89097
    ///   - rust-lang/rust#83718
    ///
    /// Reasons to have this enabled:
    /// - Check for accidental inclusion of dependencies in your API
    /// - Detect breaking changes from dependencies in your API
    pub(crate) fn deps(mut self, yes: bool) -> Self {
        self.deps = yes;
        self
    }

    /// Don't write progress to stderr
    pub(crate) fn silence(mut self, yes: bool) -> Self {
        self.silence = yes;
        self
    }

    /// Produce a rustdoc JSON file for the specified configuration.
    pub(crate) fn dump(
        &self,
        config: &mut GlobalConfig,
        manifest_path: &std::path::Path,
        pkg_spec: Option<&str>,
        all_features: bool,
    ) -> anyhow::Result<std::path::PathBuf> {
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(manifest_path)
            .no_deps()
            .exec()?;
        let manifest_target_directory = metadata
            .target_directory
            .as_path()
            .as_std_path()
            // HACK: Avoid potential errors when mixing toolchains
            .join(crate::util::SCOPE)
            .join("target");
        let target_dir = manifest_target_directory.as_path();

        let stderr = if self.silence {
            std::process::Stdio::piped()
        } else {
            // Print cargo doc progress
            std::process::Stdio::inherit()
        };

        let mut cmd = std::process::Command::new("cargo");
        cmd.env("RUSTC_BOOTSTRAP", "1")
            .env(
                "RUSTDOCFLAGS",
                "-Z unstable-options --document-private-items --document-hidden-items --output-format=json --cap-lints allow",
            )
            .stdout(std::process::Stdio::null()) // Don't pollute output
            .stderr(stderr)
            .arg("doc")
            .arg("--manifest-path")
            .arg(manifest_path)
            .arg("--target-dir")
            .arg(target_dir);
        if let Some(pkg_spec) = pkg_spec {
            cmd.arg("--package").arg(pkg_spec);
        }
        if !self.deps {
            cmd.arg("--no-deps");
        }
        if all_features {
            cmd.arg("--all-features");
        }
        if config.is_stderr_tty() {
            cmd.arg("--color=always");
        }
        let output = cmd.output()?;
        if !output.status.success() {
            if self.silence {
                anyhow::bail!(
                    "Failed when running cargo-doc on {}:\n{}",
                    manifest_path.display(),
                    String::from_utf8_lossy(&output.stderr)
                )
            } else {
                anyhow::bail!(
                    "Failed when running cargo-doc on {}. See stderr.",
                    manifest_path.display(),
                )
            }
        }

        if let Some(pkg_spec) = pkg_spec {
            // N.B.: This package spec is not necessarily part of the manifest at `manifest_path`!
            //       For example, when getting rustdoc JSON for a crate version from the registry,
            //       the manifest is "synthetic" with only a dependency on that crate and version
            //       and therefore is not a manifest *for* that crate itself.
            let crate_name = pkg_spec
                .split_once('@')
                .map(|s| s.0)
                .unwrap_or(pkg_spec)
                .to_owned();

            // N.B.: Crates named like `foo-bar` have rustdoc JSON named like `foo_bar.json`.
            let crate_name_with_underscores = crate_name.replace('-', "_");
            let json_path = target_dir.join(format!("doc/{crate_name_with_underscores}.json"));
            if json_path.exists() {
                Ok(json_path)
            } else {
                anyhow::bail!(
                    "Could not find expected rustdoc output for `{}`: {}",
                    crate_name,
                    json_path.display()
                );
            }
        } else {
            let manifest = crate::manifest::Manifest::parse(manifest_path.to_path_buf())?;

            let lib_target_name = crate::manifest::get_lib_target_name(&manifest)?;
            let json_path = target_dir.join(format!("doc/{lib_target_name}.json"));
            if json_path.exists() {
                return Ok(json_path);
            }

            let first_bin_target_name = crate::manifest::get_first_bin_target_name(&manifest)?;
            let json_path = target_dir.join(format!("doc/{first_bin_target_name}.json"));
            if !json_path.exists() {
                let crate_name = if let Some(pkg_spec) = pkg_spec {
                    pkg_spec.split_once('@').map(|s| s.0).unwrap_or(pkg_spec)
                } else {
                    crate::manifest::get_package_name(&manifest)?
                };

                anyhow::bail!(
                    "Could not find expected rustdoc output for `{}`: {}",
                    crate_name,
                    json_path.display()
                );
            }

            Ok(json_path)
        }
    }
}

impl Default for RustdocCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::GlobalConfig;
    use std::path::Path;

    use super::RustdocCommand;

    #[test]
    fn rustdoc_for_lib_crate_without_lib_section() {
        RustdocCommand::default()
            .dump(
                &mut GlobalConfig::new(),
                Path::new("./test_rustdoc/implicit_lib/Cargo.toml"),
                None,
                true,
            )
            .expect("no errors");
    }

    #[test]
    fn rustdoc_for_lib_crate_with_lib_section() {
        RustdocCommand::default()
            .dump(
                &mut GlobalConfig::new(),
                Path::new("./test_rustdoc/renamed_lib/Cargo.toml"),
                None,
                true,
            )
            .expect("no errors");
    }

    #[test]
    fn rustdoc_for_bin_crate_without_bin_section() {
        RustdocCommand::default()
            .dump(
                &mut GlobalConfig::new(),
                Path::new("./test_rustdoc/implicit_bin/Cargo.toml"),
                None,
                true,
            )
            .expect("no errors");
    }

    #[test]
    fn rustdoc_for_bin_crate_with_bin_section() {
        RustdocCommand::default()
            .dump(
                &mut GlobalConfig::new(),
                Path::new("./test_rustdoc/renamed_bin/Cargo.toml"),
                None,
                true,
            )
            .expect("no errors");
    }

    #[test]
    fn rustdoc_for_crate_in_workspace_with_workspace_manifest() {
        RustdocCommand::default()
            .dump(
                &mut GlobalConfig::new(),
                Path::new("./test_rustdoc/crate_in_workspace/Cargo.toml"),
                Some("crate_in_workspace_crate1"),
                true,
            )
            .expect("no errors");
    }

    #[test]
    fn rustdoc_for_crate_in_workspace_with_crate_manifest() {
        RustdocCommand::default()
            .dump(
                &mut GlobalConfig::new(),
                Path::new("./test_rustdoc/crate_in_workspace/crate1/Cargo.toml"),
                None,
                true,
            )
            .expect("no errors");
    }
}
