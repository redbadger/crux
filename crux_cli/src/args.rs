use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand, ValueHint::DirPath};
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
    #[command(visible_alias = "ffi")]
    Bindgen(BindgenArgs),
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

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn test_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
