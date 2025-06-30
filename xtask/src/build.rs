use anyhow::Result;
use clap::Args;
use xshell::cmd;

use crate::{Context, package_args};

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
            let package_args = &package_args(ctx);
            if self.clean {
                cmd!(ctx.sh, "{CARGO} clean").args(package_args).run()?;
            }
            cmd!(ctx.sh, "{CARGO} build --all-features")
                .args(package_args)
                .run()?;
            println!();
        }
        Ok(())
    }
}
