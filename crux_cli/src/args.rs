use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "crux",
    bin_name = "crux",
    author,
    version,
    about,
    long_about = None,
    arg_required_else_help(true),
    propagate_version = true
)]
pub(crate) struct Cli {
    /// temporary
    #[arg(long, short)]
    pub template_dir: PathBuf,
    #[command(subcommand)]
    pub command: Option<Commands>,
    #[arg(long, short, action = ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    #[command(visible_alias = "doc")]
    Doctor {
        #[arg(long, short)]
        list: bool,
        #[arg(long, short)]
        fix: Option<PathBuf>,
    },
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn test_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
