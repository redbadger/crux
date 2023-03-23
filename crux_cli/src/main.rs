use clap::Parser;
use clap_verbosity_flag::Verbosity;
use crux_cli::{produce_doc, GlobalConfig};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the Cargo.toml (for the shared library)
    #[arg(short, long)]
    manifest_path: String,

    #[command(flatten)]
    verbose: Verbosity,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut config = GlobalConfig::new().set_level(args.verbose.log_level());
    let binding = produce_doc(args.manifest_path, &mut config)?;

    let path = binding.to_str().unwrap();
    config.shell_status("generate json", format!("path: {path}"))?;

    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
