use anyhow::Result;
use std::io::{stdout, IsTerminal};
use tokio::process::Command;

use crate::{args::CodegenArgs, command_runner};

pub async fn codegen(args: &CodegenArgs) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.env("RUSTC_BOOTSTRAP", "1")
        .env(
            "RUSTDOCFLAGS",
            "-Z unstable-options --output-format=json --cap-lints=allow",
        )
        .arg("doc")
        .arg("--manifest-path")
        .arg(&args.path)
        .arg("--lib");
    if stdout().is_terminal() {
        cmd.arg("--color=always");
    }

    command_runner::run(&mut cmd).await?;

    Ok(())
}
