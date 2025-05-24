use anyhow::Result;
use clap::Args;
use xshell::cmd;

use crate::Context;

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
            cmd!(ctx.sh, "{CARGO} nextest run --all-features").run()?;
            if self.doc {
                cmd!(ctx.sh, "{CARGO} test --doc --all-features").run()?;
            }
            println!();
        }
        Ok(())
    }
}
