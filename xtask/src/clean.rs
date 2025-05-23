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
            let _dir = ctx.sh.push_dir(dir);
            println!("~ {}", dir.display());
            cmd!(ctx.sh, "{CARGO} clean").run()?;
            if self.generated {
                cmd!(ctx.sh, "rm -rf */generated").run()?;
            }
            println!();
        }
        Ok(())
    }
}
