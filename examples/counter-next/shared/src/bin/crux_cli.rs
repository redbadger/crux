use std::env;

use uniffi::deps::anyhow;

fn main() -> anyhow::Result<()> {
    env::set_var("CRATE_NAME", env!("CARGO_PKG_NAME"));
    crux_core::run_cli()
}
