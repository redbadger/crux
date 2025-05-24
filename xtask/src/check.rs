use anyhow::Result;
use clap::Args;
use xshell::cmd;

use crate::Context;

const CARGO: &str = crate::CARGO;

#[derive(Args)]
pub(crate) struct Check {
    #[arg(short, long)]
    pub(crate) clippy: bool,
}
impl Check {
    pub(crate) fn run(&self, ctx: &Context) -> Result<()> {
        println!("Check...");
        for dir in &ctx.workspaces {
            let _dir = ctx.push_dir(dir);
            cmd!(ctx.sh, "{CARGO} check --all-features").run()?;
            if self.clippy {
                cmd!(
                    ctx.sh,
                    "{CARGO} clippy -- --no-deps -Dclippy::pedantic -Dwarnings"
                )
                .run()?;
            }
            println!();
        }
        Ok(())
    }
}
