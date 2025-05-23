use anyhow::Result;
use xshell::cmd;

use crate::Context;

const CARGO: &str = crate::CARGO;

pub(crate) fn run(ctx: &Context) -> Result<()> {
    println!("Check...");
    for dir in &ctx.workspaces {
        let _dir = ctx.sh.push_dir(dir);
        println!("~ {}", dir.display());
        cmd!(ctx.sh, "{CARGO} check --all-features").run()?;
        println!();
    }
    Ok(())
}
