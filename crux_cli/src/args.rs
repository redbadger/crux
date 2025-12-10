use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand, ValueHint::DirPath};
use convert_case::{Case, Casing, Pattern, delim_boundary};
use derive_builder::Builder;

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
    #[arg(long, short, value_name = "STRING", default_value = "shared")]
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

    /// Specify a package name for each language you want to generate code for.
    #[command(flatten)]
    pub generate: Generate,
}

#[derive(Args)]
#[group(required = true, multiple = true)]
pub struct Generate {
    /// Java package name.
    /// If not specified, no code will be generated for Java/Kotlin.
    #[arg(
        long,
        short,
        value_name = "dotted.case",
        value_parser = dot_case
    )]
    pub java: Option<String>,

    /// Swift package name.
    /// If not specified, no code will be generated for Swift.
    #[arg(
        long,
        short,
        value_name = "PascalCase",
        value_parser = pascal_case
    )]
    pub swift: Option<String>,

    /// TypeScript package name.
    /// If not specified, no code will be generated for TypeScript.
    #[arg(
        long,
        short,
        value_name = "snake_case",
        value_parser = snake_case
    )]
    pub typescript: Option<String>,
}

#[derive(Clone, Args, Builder)]
#[builder(build_fn(validate = "Self::validate"))]
pub struct BindgenArgs {
    /// Package name of the crate containing your Crux App
    #[arg(long, short, value_name = "STRING", default_value = "shared")]
    pub crate_name: String,

    #[clap(flatten)]
    pub languages: BindgenLanguages,
}

#[derive(Clone, Default, Args)]
#[group(required = true, multiple = true)]
pub struct BindgenLanguages {
    /// Generate bindings for Kotlin, and output to the specified path
    #[arg(long, short, value_name = "DIR", value_hint = DirPath)]
    pub kotlin: Option<PathBuf>,

    /// Generate bindings for Swift, and output to the specified path
    #[arg(long, short, value_name = "DIR", value_hint = DirPath)]
    pub swift: Option<PathBuf>,
}

impl BindgenArgsBuilder {
    pub fn kotlin(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.languages.get_or_insert_default().kotlin = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn swift(&mut self, path: impl AsRef<Path>) -> &mut Self {
        self.languages.get_or_insert_default().swift = Some(path.as_ref().to_path_buf());
        self
    }

    fn validate(&self) -> Result<(), String> {
        const ERROR: &str = "call kotlin() and/or swift() to generate bindings";

        let languages = self.languages.as_ref().ok_or_else(|| ERROR.to_string())?;

        if languages.kotlin.is_none() && languages.swift.is_none() {
            Err(ERROR.to_string())
        } else {
            Ok(())
        }
    }
}

fn dot_case(s: &str) -> Result<String, String> {
    const DOT_CASE: Case = Case::Custom {
        boundaries: &[delim_boundary!(".")],
        pattern: Pattern::Lowercase,
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
            dot_case("com.example.crux.shared.types").unwrap(),
            "com.example.crux.shared.types"
        );
        assert_eq!(
            dot_case("comExampleCruxSharedTypes").unwrap_err(),
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
