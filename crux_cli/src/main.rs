use std::{env, fs};

use anyhow::Result;
use args::Commands;
use clap::Parser;
use ramhorns::{Content, Template};
use walkdir::WalkDir;

use crate::args::Cli;

mod args;
mod config;
mod workspace;

#[derive(Content)]
struct CoreContext {
    workspace: String,
    name: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Doctor { .. }) => {
            let workspace = workspace::read_config()?;

            for (name, _core) in &workspace.cores {
                // let root = env::current_dir()?.join(&core.source);
                let template_root = env::current_dir()?.join(&cli.template_dir);
                for entry in WalkDir::new(template_root)
                    .contents_first(true)
                    .sort_by_file_name()
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_dir() {
                        continue;
                    }
                    let ctx = CoreContext {
                        workspace: workspace.name.to_ascii_lowercase().replace(" ", "_"),
                        name: name.clone(),
                    };
                    let template = fs::read_to_string(entry.path())?;
                    let template = Template::new(template).unwrap();
                    let rendered = template.render(&ctx);
                    println!(
                        "------- {} --------",
                        entry.path().file_name().unwrap().to_string_lossy()
                    );
                    println!("{}", rendered);
                }
            }
            workspace::write_config(&workspace)
        }
        None => Ok(()),
    }
}
