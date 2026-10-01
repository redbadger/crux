use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use crux_core::type_generation::facet::{Config, TypeRegistry};
use log::info;

use shared::Counter;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Language {
    Swift,
    Kotlin,
    Typescript,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_enum)]
    language: Language,
    #[arg(short, long)]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    // No `.boltffi(..)` here. `CoreFfi::new` takes the shell's `CruxShell`
    // callback, which the middleware uses to deliver effects after `update`
    // or `resolve` has returned, and the generated `FfiBridge` constructs
    // `CoreFfi` with no arguments. So each shell writes the three-method
    // `CoreBridge` over its own `CoreFfi` and builds `Core` from that.
    let typegen_app = TypeRegistry::new().register_app::<Counter>()?.build()?;

    let name = match args.language {
        Language::Swift => "App",
        Language::Kotlin => "com.crux.examples.counter.middleware",
        Language::Typescript => "app",
    };
    let config = Config::builder(name, &args.output_dir).build();

    match args.language {
        Language::Swift => {
            info!("Typegen for Swift");
            typegen_app.swift(&config)?;
        }
        Language::Kotlin => {
            info!("Typegen for Kotlin");
            typegen_app.kotlin(&config)?;
        }
        Language::Typescript => {
            info!("Typegen for TypeScript");
            typegen_app.typescript(&config)?;
        }
    }

    Ok(())
}
