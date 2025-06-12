use uniffi::deps::anyhow;

fn main() -> anyhow::Result<()> {
    crux_core::cli::run(Some(env!("CARGO_PKG_NAME")))
}
