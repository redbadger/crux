mod lsp;
mod lsp_ext;

use anyhow::Result;
use clap::Parser;

/// Crux CLI
#[derive(Parser, Debug)]
#[clap(version)]
enum Command {
    /// Run the language server.
    #[clap(name = "lsp")]
    LanguageServer,
}

fn main() -> Result<()> {
    match Command::parse() {
        Command::LanguageServer => lsp::run(),
    }?;
    Ok(())
}
