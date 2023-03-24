use std::collections::BTreeMap;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use crux_cli::{parse_crate, GlobalConfig};
use trustfall::TransparentValue;
use trustfall_rustdoc::{VersionedIndexedCrate, VersionedRustdocAdapter};

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
    let types = parse_crate(args.manifest_path, &mut config)?;
    let indexed_crate = VersionedIndexedCrate::new(&types);

    let adapter = VersionedRustdocAdapter::new(&indexed_crate, Some(&indexed_crate))
        .expect("failed to create adapter");
    let args: BTreeMap<String, TransparentValue> = BTreeMap::new();
    let results_iter = adapter.run_query(include_str!("./queries/implements_app.gql"), args)?;
    let actual_results: Vec<BTreeMap<_, _>> = results_iter
        .map(|res| res.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
        .collect();

    config.shell_status("found", format!("types: {actual_results:#?}"))?;

    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
