use anyhow::Result;
use clap::Args;
use xshell::cmd;

use crate::Context;

const CARGO: &str = crate::CARGO;

#[derive(Args)]
pub(crate) struct Clean {
    #[arg(short, long)]
    pub(crate) generated: bool,
}

impl Clean {
    pub(crate) fn run(&self, ctx: &Context) -> Result<()> {
        println!("Clean...");
        for dir in &ctx.workspaces {
            let _dir = ctx.push_dir(dir);
            cmd!(ctx.sh, "{CARGO} clean").run()?;
            if self.generated {
                cmd!(ctx.sh, "echo rm -rf */generated").run()?;
            }
            println!();
        }
        Ok(())
    }
}
