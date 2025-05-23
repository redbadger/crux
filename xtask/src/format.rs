use anyhow::Result;
use clap::Args;
use xshell::cmd;

use crate::Context;

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
            let _dir = ctx.sh.push_dir(dir);
            println!("~ {}", dir.display());
            let args = if self.fix { None } else { Some("--check") };
            cmd!(ctx.sh, "{CARGO} fmt --all {args...}").run()?;
            println!();
        }
        Ok(())
    }
}
