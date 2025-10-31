use anyhow::Result;
use clap::Args;
use fs_walk::WalkOptions;
use xshell::cmd;

use crate::{Context, package_args};

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
            let package_args = &package_args(ctx);
            cmd!(ctx.sh, "{CARGO} clean").args(package_args).run()?;
            if self.generated {
                for file in WalkOptions::new()
                    .dirs()
                    .max_depth(2)
                    .name("generated")
                    .name("node_modules")
                    .walk(dir)
                    .flatten()
                {
                    println!("Removing {}", file.display());
                    ctx.sh.remove_path(&file)?;
                }
            }
            println!();
        }
        Ok(())
    }
}
