mod build;
mod check;
mod clean;
mod format;
mod publish;
mod test;

use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Result, anyhow};
use build::Build;
use cargo_metadata::MetadataCommand;
use check::Check;
use clap::{Args, Parser, Subcommand};
use clean::Clean;
use format::Format;
use human_repr::HumanDuration;
use ignore::WalkBuilder;
use publish::Publish;
use test::Test;
use xshell::{PushDir, Shell};

const CARGO: &str = env!("CARGO");

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    scope: Scope,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Args)]
#[group(required = false, multiple = false)]
struct Scope {
    /// finds all the workspaces in the repository, and performs the given command on each
    #[arg(short, long)]
    all: bool,

    /// performs the given command on the specified packages in the root workspace
    #[arg(short, long)]
    package: Option<Vec<String>>,
}

#[derive(Subcommand)]
enum Commands {
    None,
    /// Build the root workspace (or all workspaces if --all), optionally cleaning before building
    Build(Build),
    /// Check the root workspace (or all workspaces if --all), with optional clippy pedantic checks
    Check(Check),
    /// Clean the root workspace (or all workspaces if --all), optionally removing generated code
    Clean(Clean),
    /// Format the root workspace (or all workspaces if --all), optionally fixing code where possible
    Format(Format),
    /// Publish the root workspace (or all workspaces if --all), defaults to `--dry-run`, specify `--yes` to publish
    Publish(Publish),
    /// Test the root workspace (or all workspaces if --all), optionally running doc tests
    Test(Test),
    /// Run the relevant commands (to match CI) on the root workspace (or all workspaces if --all)
    CI,
}

struct Context {
    sh: Shell,
    workspaces: Vec<PathBuf>,
    packages: Vec<String>,
}

impl Context {
    pub fn push_dir<P>(&self, path: P) -> PushDir<'_>
    where
        P: AsRef<Path>,
    {
        println!("~ {}", path.as_ref().display());
        self.sh.push_dir(path)
    }
}

fn main() -> Result<()> {
    let start = Instant::now();
    let cli = Cli::parse();

    let sh = Shell::new()?;
    let project_root = project_root()?;
    sh.change_dir(&project_root);

    let workspaces = if cli.scope.all {
        workspaces()?
    } else {
        vec![project_root]
    };

    let packages = cli.scope.package.unwrap_or_default();

    let ctx = Context {
        sh,
        workspaces,
        packages,
    };
    println!("Workspace: {:?}", ctx.workspaces);
    println!("Packages: {:?}", ctx.packages);

    match &cli.command {
        Commands::None => anyhow::Ok(())?,
        Commands::Build(build) => build.run(&ctx)?,
        Commands::Check(check) => check.run(&ctx)?,
        Commands::Clean(clean) => clean.run(&ctx)?,
        Commands::Format(format) => format.run(&ctx)?,
        Commands::Publish(publish) => publish.run(&ctx)?,
        Commands::Test(test) => test.run(&ctx)?,
        Commands::CI => {
            // Clean { generated: true }.run(&ctx)?;
            Format { fix: false }.run(&ctx)?;
            Check { clippy: true }.run(&ctx)?;
            Build { clean: false }.run(&ctx)?;
            Test { doc: true }.run(&ctx)?;
        }
    }

    println!("Total time: {}", start.elapsed().human_duration());
    Ok(())
}

fn project_root() -> Result<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow!("can't find project root"))?
        .to_path_buf();
    Ok(root)
}

fn workspaces() -> Result<Vec<PathBuf>> {
    println!("Finding workspaces...");
    let mut workspaces = vec![];
    let mut checked: HashSet<PathBuf> = HashSet::new();

    for entry in WalkBuilder::new(&project_root()?)
        .max_depth(Some(2))
        .build()
        .filter_map(Result::ok)
    {
        let dir = entry.into_path();
        if dir.is_dir() && !checked.contains(&dir) {
            let manifest = dir.join("Cargo.toml");
            if manifest.exists() {
                let metadata = MetadataCommand::new().manifest_path(&manifest).exec()?;
                if metadata.workspace_root == dir {
                    println!("found workspace: {}", dir.display());
                    workspaces.push(dir.clone());
                    for member in metadata
                        .workspace_members
                        .iter()
                        .filter_map(|id| metadata[id].manifest_path.parent())
                    {
                        checked.insert(member.into());
                    }
                }
            }
        }
    }
    workspaces.sort();
    Ok(workspaces)
}

fn package_args(ctx: &Context) -> Vec<&str> {
    ctx.packages
        .iter()
        .flat_map(|p| vec!["--package", p])
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
