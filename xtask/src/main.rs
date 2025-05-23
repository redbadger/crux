mod build;
mod check;
mod clean;
mod format;
mod test;

use std::{env, path::PathBuf, time::Instant};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use human_repr::HumanDuration;
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
    Build(build::Build),
    Check,
    Clean(clean::Clean),
    Format(format::Format),
    Test(test::Test),
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
        Commands::Check => check::run(&ctx)?,
        Commands::Clean(clean) => clean.run(&ctx)?,
        Commands::Format(format) => format.run(&ctx)?,
        Commands::Test(test) => test.run(&ctx)?,
        Commands::CI => {
            clean::Clean { generated: true }.run(&ctx)?;
            format::Format { fix: false }.run(&ctx)?;
            build::Build { clean: false }.run(&ctx)?;
            test::Test { doc: true }.run(&ctx)?;
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
    let root = project_root()?;
    let mut workspaces = vec![root.clone()];

    let examples = root.join("examples");
    for example in examples.read_dir()? {
        let example = example?.path();
        if example.is_dir() {
            workspaces.push(example);
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
