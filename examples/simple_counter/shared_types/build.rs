use crux_core::typegen::TypeGen;
use shared::Counter;
use std::{path::PathBuf, process::Command};

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut typegen = TypeGen::new();

    typegen.register_app::<Counter>()?;

    let output_root = PathBuf::from("./generated");

    typegen.swift("SharedTypes", output_root.join("swift"))?;

    typegen.java("com.crux.example.simple_counter", output_root.join("java"))?;

    typegen.typescript("shared_types", output_root.join("typescript"))?;

    let module_name = "SharedTypes";
    let path = output_root.join("csharp");

    gen.csharp(module_name, &path)?;

    let package_path = module_name.replace('.', "/");

    let mut command = Command::new("dotnet");
    command.arg("build");
    command.arg("-c");
    command.arg("Release");
    command.current_dir(path.join(&package_path));

    // Execute the command and wait for it to finish, capturing output
    let output = command.spawn()?.wait_with_output()?;

    if !output.status.success() {
        anyhow::bail!("dotnet build command failed");
    }

    Ok(())
}
