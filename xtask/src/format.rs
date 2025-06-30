use anyhow::Result;
use clap::Args;
use xshell::cmd;

use crate::{Context, package_args};

const CARGO: &str = crate::CARGO;

#[derive(Args)]
pub(crate) struct Format {
    #[arg(short, long)]
    pub(crate) fix: bool,
}

impl Format {
    pub(crate) fn run(&self, ctx: &Context) -> Result<()> {
        println!("Format...");
        for dir in &ctx.workspaces {
            let _dir = ctx.push_dir(dir);
            let args = if self.fix { None } else { Some("--check") };
            let package_args = &package_args(ctx);
            cmd!(ctx.sh, "{CARGO} fmt --all {args...}")
                .args(package_args)
                .run()?;
            println!();
        }
        Ok(())
    }
}
