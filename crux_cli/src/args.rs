use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};

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
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, short, action = ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(visible_alias = "doc")]
    Doctor(DoctorArgs),

    #[command(visible_alias = "gen")]
    Codegen(CodegenArgs),
}

#[derive(Args)]
pub struct DoctorArgs {
    #[arg(long, short)]
    pub fix: Option<PathBuf>,

    #[arg(long, short, default_value = "false")]
    pub include_source_code: bool,

    /// temporary
    #[arg(long, short)]
    pub template_dir: PathBuf,

    #[arg(long, short)]
    pub path: Option<PathBuf>,
}

#[derive(Args)]
pub struct CodegenArgs {
    /// name of the library containing your Crux App
    #[arg(long, short)]
    pub lib: String,
    /// Optional output directory for generated code
    #[arg(long, short)]
    pub output: Option<PathBuf>,
    /// Optional Java package name
    #[arg(long, short)]
    pub java_package: Option<String>,
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn test_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
