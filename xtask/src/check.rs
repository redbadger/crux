use anyhow::Result;
use clap::Args;
use xshell::cmd;

use crate::{package_args, Context};

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
            let package_args = &package_args(ctx);
            cmd!(ctx.sh, "{CARGO} check --all-features")
                .args(package_args)
                .run()?;
            if self.clippy {
                cmd!(ctx.sh, "{CARGO} clippy")
                    .args(package_args)
                    .args(vec!["--", "--no-deps", "-Dclippy::pedantic", "-Dwarnings"])
                    .run()?;
            }
            println!();
        }
        Ok(())
    }
}
