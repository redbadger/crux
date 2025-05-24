use anyhow::Result;
use clap::Args;
use xshell::cmd;

use crate::Context;

const CARGO: &str = crate::CARGO;

#[derive(Args)]
pub(crate) struct Build {
    #[arg(short, long)]
    pub(crate) clean: bool,
}

impl Build {
    pub(crate) fn run(&self, ctx: &Context) -> Result<()> {
        println!("Build...");
        for dir in &ctx.workspaces {
            let _dir = ctx.push_dir(dir);
            if self.clean {
                cmd!(ctx.sh, "{CARGO} clean").run()?;
            }
            cmd!(ctx.sh, "{CARGO} build --all-features").run()?;
            println!();
        }
        Ok(())
    }
}
