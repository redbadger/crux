use crux_core::typegen::TypeGen;
use crux_kv::error::KeyValueError;
use shared::{
    favorites::events::{FavoritesEvent, FavoritesState},
    weather::{events::WeatherEvent, model::CurrentResponse},
    App, Event, Workflow, WorkflowViewModel,
};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    // Register the app first
    gen.register_app::<App>()?;

    // Register all enum types that need to be shared
    gen.register_type::<Event>()?;
    gen.register_type::<Workflow>()?;
    gen.register_type::<WorkflowViewModel>()?;
    gen.register_type::<FavoritesEvent>()?;
    gen.register_type::<WeatherEvent>()?;
    gen.register_type::<FavoritesState>()?;
    gen.register_type::<KeyValueError>()?;

    // Register sample data
    let _ = gen.register_samples(vec![CurrentResponse::default()]);

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;
    gen.java("com.crux.example.counter", output_root.join("java"))?;
    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
