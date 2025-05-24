mod build;
mod check;
mod clean;
mod format;
mod test;

use std::{collections::HashSet, env, path::PathBuf, time::Instant};

use anyhow::{anyhow, Result};
use build::Build;
use cargo_metadata::MetadataCommand;
use check::Check;
use clap::{Parser, Subcommand};
use clean::Clean;
use format::Format;
use human_repr::HumanDuration;
use ignore::WalkBuilder;
use test::Test;
use xshell::Shell;

const CARGO: &str = env!("CARGO");

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    all: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build(Build),
    Check(Check),
    Clean(Clean),
    Format(Format),
    Test(Test),
    CI,
}

struct Context {
    sh: Shell,
    workspaces: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let start = Instant::now();
    let cli = Cli::parse();

    let sh = Shell::new()?;
    sh.change_dir(project_root()?);

    let workspaces = if cli.all {
        workspaces()?
    } else {
        vec![project_root()?]
    };

    let ctx = Context { sh, workspaces };

    match &cli.command {
        Commands::Build(build) => build.run(&ctx)?,
        Commands::Check(check) => check.run(&ctx)?,
        Commands::Clean(clean) => clean.run(&ctx)?,
        Commands::Format(format) => format.run(&ctx)?,
        Commands::Test(test) => test.run(&ctx)?,
        Commands::CI => {
            Clean { generated: true }.run(&ctx)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
