use uniffi::deps::anyhow;

fn main() -> anyhow::Result<()> {
    if cfg!(feature = "cli") {
        crux_core::run_cli()
    } else {
        Ok(())
    }
}
