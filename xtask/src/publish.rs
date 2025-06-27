use std::collections::HashMap;

use anyhow::{Result, bail};
use cargo_metadata::{Metadata, MetadataCommand};
use clap::Args;
use xshell::cmd;

use crate::Context;

const CARGO: &str = crate::CARGO;

// in order for publishing
const PACKAGES: &[&str] = &[
    "crux_cli",
    "crux_macros",
    "crux_core",
    "crux_http",
    "crux_kv",
    "crux_platform",
    "crux_time",
];

#[derive(Args)]
pub(crate) struct Publish {
    #[arg(short, long)]
    pub(crate) yes: bool,
}

impl Publish {
    pub(crate) fn run(&self, ctx: &Context) -> Result<()> {
        if ctx.workspaces.len() != 1 {
            // first workspace is the root
            bail!("publishing is only supported for the root workspace");
        }
        let project_root = &ctx.workspaces[0];
        let manifest = project_root.join("Cargo.toml");
        let metadata = MetadataCommand::new().manifest_path(&manifest).exec()?;
        let versions = versions(&metadata);
        let packages = if ctx.packages.is_empty() {
            PACKAGES.to_vec()
        } else {
            ctx.packages
                .iter()
                .map(std::string::String::as_str)
                .collect()
        };
        for pkg in packages {
            let version = &versions[pkg];
            println!("Publishing {pkg} at version {version}...");
            let _dir = ctx.push_dir(pkg);
            let dry_run = if self.yes { None } else { Some("--dry-run") };
            cmd!(ctx.sh, "{CARGO} publish --package {pkg} {dry_run...}").run()?;
            if self.yes {
                let tag = format!("{pkg}-v{version}");
                cmd!(ctx.sh, "git tag {tag}").run()?;
                cmd!(ctx.sh, "git push origin tag {tag}").run()?;
            }
            println!();
        }
        Ok(())
    }
}

fn versions(metadata: &Metadata) -> HashMap<&str, String> {
    metadata
        .workspace_members
        .iter()
        .map(|id| {
            let package = metadata
                .packages
                .iter()
                .find(|p| &p.id == id)
                .expect("package to be found");
            (package.name.as_str(), package.version.to_string())
        })
        .collect()
}
