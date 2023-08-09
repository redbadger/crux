use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "crux",bin_name = "crux", author, version, about, long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    Doctor {
        #[arg(long, short)]
        list: bool,
        #[arg(long, short)]
        fix: Option<std::path::PathBuf>,
    },
}
