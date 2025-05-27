use anyhow::Result;
use clap::Args;
use xshell::cmd;

use crate::{package_args, Context};

const CARGO: &str = crate::CARGO;

#[derive(Args)]
pub(crate) struct Test {
    #[arg(short, long)]
    pub(crate) doc: bool,
}

impl Test {
    pub(crate) fn run(&self, ctx: &Context) -> Result<()> {
        println!("Test...");
        for dir in &ctx.workspaces {
            let _dir = ctx.push_dir(dir);
            let package_args = &package_args(ctx);
            cmd!(ctx.sh, "{CARGO} nextest run --all-features")
                .args(package_args)
                .run()?;
            if self.doc {
                cmd!(ctx.sh, "{CARGO} test --doc --all-features")
                    .args(package_args)
                    .run()?;
            }
            println!();
        }
        Ok(())
    }
}
