use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueHint::DirPath};
use heck::{ToPascalCase, ToSnakeCase};

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
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(visible_alias = "gen")]
    Codegen(CodegenArgs),
}

#[derive(Args)]
pub struct CodegenArgs {
    /// name of the library containing your Crux App
    #[arg(long, short, value_name = "STRING")]
    pub lib: String,
    /// Output directory for generated code
    #[arg(
        long,
        short,
        value_name = "DIR",
        value_hint = DirPath,
        default_value = "./shared/generated",
    )]
    pub output: PathBuf,
    /// Java package name
    #[arg(
        long,
        short,
        value_name = "dotted.case",
        value_parser = dotted_case,
        default_value = "com.crux.example.shared.types"
    )]
    pub java_package: String,
    /// Swift package name
    #[arg(
        long,
        short,
        value_name = "PascalCase",
        value_parser = pascal_case,
        default_value = "SharedTypes")]
    pub swift_package: String,
    /// TypeScript package name
    #[arg(
        long,
        short,
        value_name = "snake_case",
        value_parser = snake_case,
        default_value = "shared_types")]
    pub typescript_package: String,
}

fn dotted_case(s: &str) -> Result<String, String> {
    if s == s.to_snake_case().replace('_', ".") {
        Ok(s.to_string())
    } else {
        Err(format!("Invalid dotted case: {}", s))
    }
}

fn pascal_case(s: &str) -> Result<String, String> {
    if s == s.to_pascal_case() {
        Ok(s.to_string())
    } else {
        Err(format!("Invalid pascal case: {}", s))
    }
}

fn snake_case(s: &str) -> Result<String, String> {
    if s == s.to_snake_case() {
        Ok(s.to_string())
    } else {
        Err(format!("Invalid snake case: {}", s))
    }
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
