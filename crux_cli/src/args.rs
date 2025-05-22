use std::path::PathBuf;

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand, ValueHint::DirPath};
use convert_case::{pattern, Boundary, Case, Casing};

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
    #[command(visible_alias = "ffi")]
    Bindgen(BindgenArgs),
}

#[derive(Args)]
pub struct CodegenArgs {
    /// name of the library containing your Crux App
    #[arg(long, short, value_name = "STRING", env)]
    pub crate_name: String,

    /// Output directory for generated code
    #[arg(
        long,
        short,
        value_name = "DIR",
        value_hint = DirPath,
        default_value = "./shared/generated",
    )]
    pub out_dir: PathBuf,

    /// Java package name
    #[arg(
        long,
        short,
        value_name = "dotted.case",
        value_parser = dotted_case,
        default_value = "com.crux.example.shared"
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

#[derive(Args)]
pub struct BindgenArgs {
    /// name of the crate containing your Crux App
    #[arg(long, short, value_name = "STRING", env)]
    pub crate_name: String,

    // library path (target/debug/libshared.so)
    /// Output directory for generated code
    #[arg(
        long,
        short,
        value_name = "DIR",
        value_hint = DirPath,
        default_value = "./shared/generated",
    )]
    pub out_dir: Utf8PathBuf,
}

fn dotted_case(s: &str) -> Result<String, String> {
    const DOT_CASE: Case = Case::Custom {
        boundaries: &[Boundary::from_delim(".")],
        pattern: pattern::lowercase,
        delim: ".",
    };
    if s.is_case(DOT_CASE) {
        Ok(s.to_string())
    } else {
        Err(format!("Invalid dotted case: {s}"))
    }
}

fn pascal_case(s: &str) -> Result<String, String> {
    if s.is_case(Case::Pascal) {
        Ok(s.to_string())
    } else {
        Err(format!("Invalid pascal case: {s}"))
    }
}

fn snake_case(s: &str) -> Result<String, String> {
    if s.is_case(Case::Snake) {
        Ok(s.to_string())
    } else {
        Err(format!("Invalid snake case: {s}"))
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

    #[test]
    fn dotted() {
        assert_eq!(
            dotted_case("com.example.crux.shared.types").unwrap(),
            "com.example.crux.shared.types"
        );
        assert_eq!(
            dotted_case("comExampleCruxSharedTypes").unwrap_err(),
            "Invalid dotted case: comExampleCruxSharedTypes"
        );
    }

    #[test]
    fn pascal() {
        assert_eq!(pascal_case("SharedTypes").unwrap(), "SharedTypes");
        assert_eq!(
            pascal_case("shared_types").unwrap_err(),
            "Invalid pascal case: shared_types"
        );
    }

    #[test]
    fn snake() {
        assert_eq!(snake_case("shared_types").unwrap(), "shared_types");
        assert_eq!(
            snake_case("SharedTypes").unwrap_err(),
            "Invalid snake case: SharedTypes"
        );
    }
}
