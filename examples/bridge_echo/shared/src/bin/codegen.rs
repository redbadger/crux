use std::path::PathBuf;

use crux_core::{
    cli::{bindgen, BindgenArgsBuilder},
    type_generation::facet::{Config, TypeRegistry},
};
use log::info;
use uniffi::deps::anyhow::Result;

use shared::App;

fn main() -> Result<()> {
    pretty_env_logger::init();

    let out_dir = PathBuf::from("./shared/generated");

    info!("Generating types for App");

    let typegen_app = TypeRegistry::new().register_app::<App>()?.build()?;

    typegen_app.swift(
        &Config::builder("App", out_dir.join("swift"))
            .add_extensions()
            .add_runtimes()
            .build(),
    )?;

    typegen_app.kotlin(
        &Config::builder("com.crux.example.bridge_echo", out_dir.join("kotlin"))
            .add_extensions()
            .add_runtimes()
            .build(),
    )?;

    typegen_app.typescript(
        &Config::builder("app", out_dir.join("typescript"))
            .add_extensions()
            .add_runtimes()
            .build(),
    )?;

    // bindgen for kotlin only - swift bindgen is done in /.build.sh by `cargo swift`
    bindgen(
        &BindgenArgsBuilder::default()
            .crate_name(env!("CARGO_PKG_NAME").to_string())
            .kotlin(out_dir.join("kotlin"))
            .build()?,
    )
}
