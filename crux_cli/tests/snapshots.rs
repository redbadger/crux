use anyhow::{Context, Result};
use crux_cli::{CodegenArgs, Generate};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};
use tempdir::TempDir;

static TESTS_FOLDER_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"));

#[test]
fn test_snapshots() -> Result<()> {
    for directory in TESTS_FOLDER_PATH.read_dir()? {
        let path = directory?.path();
        if path.is_dir() {
            let test = path.file_name().context("no test name")?.to_string_lossy();

            let dir = TempDir::new(&test)?;
            let tempdir = dir.path();

            fs::copy(path.join("Cargo.toml"), tempdir.join("Cargo.toml"))?;
            copy_dir_all(path.join("src"), tempdir.join("src"))?;

            let current_dir = env::current_dir()?;
            env::set_current_dir(tempdir)?;
            crux_cli::codegen::codegen(&CodegenArgs {
                crate_name: "shared".to_string(),
                out_dir: PathBuf::from(tempdir).join("generated"),
                generate: Generate {
                    java: Some("com.crux.example.shared".to_string()),
                    swift: Some("SharedTypes".to_string()),
                    typescript: Some("shared_types".to_string()),
                },
            })?;
            env::set_current_dir(current_dir)?;

            let generated_dir = path.join("generated");

            let swift_dir = generated_dir.join("swift");
            fs::create_dir_all(&swift_dir)?;
            for entry in
                fs::read_dir(tempdir.join("generated/swift/SharedTypes/Sources/SharedTypes"))?
            {
                let entry = entry?;
                let ty = entry.file_type()?;
                if ty.is_file() {
                    let actual = fs::read_to_string(entry.path())?;
                    let expected = swift_dir.join(entry.file_name());
                    expect_test::expect_file![expected].assert_eq(&actual);
                }
            }

            let java_dir = generated_dir.join("java");
            fs::create_dir_all(&java_dir)?;
            for entry in fs::read_dir(tempdir.join("generated/java/com/crux/example/shared"))? {
                let entry = entry?;
                let ty = entry.file_type()?;
                if ty.is_file() {
                    let actual = fs::read_to_string(entry.path())?;
                    let expected = java_dir.join(entry.file_name());
                    expect_test::expect_file![expected].assert_eq(&actual);
                }
            }

            let typescript_dir = generated_dir.join("typescript");
            fs::create_dir_all(&typescript_dir)?;
            for entry in fs::read_dir(tempdir.join("generated/typescript/types"))? {
                let entry = entry?;
                let ty = entry.file_type()?;
                if ty.is_file() {
                    let actual = fs::read_to_string(entry.path())?;
                    let expected = typescript_dir.join(entry.file_name());
                    expect_test::expect_file![expected].assert_eq(&actual);
                }
            }

            dir.close()?;
        }
    }

    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }

    Ok(())
}
