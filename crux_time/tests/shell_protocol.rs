//! The protocol of `crux_time`, generated for an app and compiled.
//!
//! The app uses `NotifyAfter` and registers no shell extension.
//! Type generation must still emit the protocol of `crux_time` and every type
//! that it names, and the generated package must compile.
//!
//! `cargo test` cannot compile Swift, Kotlin, TypeScript or C#, so each test
//! gives the generated package to the toolchain on `PATH`. A missing toolchain
//! is a printed skip. Set `CRUX_REQUIRE_SHELL_TOOLCHAINS` to make it a
//! failure.
#![cfg(feature = "facet_typegen")]

use std::{
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crux_core::{
    Command as CruxCommand,
    macros::effect,
    render::RenderOperation,
    type_generation::facet::{CodeGenerator, Config, TypeRegistry},
};
use crux_time::operation::NotifyAfter;
use facet::Facet;

#[derive(Facet)]
#[repr(C)]
pub enum Event {
    None,
}

#[derive(Facet)]
pub struct ViewModel;

/// An app that uses some of the operations of the capability.
#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    TimeNotifyAfter(NotifyAfter),
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Event = Event;
    type Model = ();
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, _event: Event, _model: &mut Self::Model) -> CruxCommand<Effect, Event> {
        CruxCommand::done()
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {
        ViewModel
    }
}

/// The name of the companion file, and a line of the protocol in each
/// language.
const SWIFT: (&str, &str) = ("Time.swift", "public protocol TimeHandler: Sendable {");
const KOTLIN: (&str, &str) = ("Time.kt", "interface TimeHandler {");
const TYPESCRIPT: &str = "export interface TimeHandler {";
const CSHARP: (&str, &str) = ("Time.cs", "public interface ITimeHandler");

/// A type of an operation that the app does not send.
const UNSENT: &str = "ClearTimer";

/// The app, with no shell extension and no other registration.
fn generator() -> CodeGenerator {
    let mut registry = TypeRegistry::new();
    registry
        .register_app::<App>()
        .expect("should register the app");
    registry.build().expect("should build the registry")
}

/// Whether a toolchain that this test cannot find is a failure, not a skip.
fn toolchains_required() -> bool {
    std::env::var_os("CRUX_REQUIRE_SHELL_TOOLCHAINS").is_some_and(|value| !value.is_empty())
}

/// Reports a toolchain that the test did not find: a failure when
/// `CRUX_REQUIRE_SHELL_TOOLCHAINS` is set, and a printed skip if not.
fn unavailable(missing: &str, consequence: &str) {
    assert!(
        !toolchains_required(),
        "{missing}, so {consequence}. CRUX_REQUIRE_SHELL_TOOLCHAINS is set, so a missing \
         toolchain fails rather than skips: install it, or unset the variable."
    );
    println!("skipping: {missing}, so {consequence}");
}

/// Runs `program` in `dir` and asserts that it succeeded. A program that is
/// not on `PATH` goes to [`unavailable`].
fn build<S: AsRef<OsStr>>(program: &str, args: &[S], dir: &Path) {
    match Command::new(program).current_dir(dir).args(args).output() {
        Ok(output) => assert!(
            output.status.success(),
            "`{program}` failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            unavailable(
                &format!("`{program}` is not on PATH"),
                "the generated protocol is not compiled",
            );
        }
        Err(e) => panic!("could not run `{program}`: {e}"),
    }
}

/// Finds a `kotlinx-coroutines-core` jar, which the generated Kotlin imports.
/// Gradle and Maven keep one under the home directory after a build that uses
/// it. Set `KOTLINX_COROUTINES_JAR` to use a jar in a different location.
fn coroutines_jar() -> Option<PathBuf> {
    if let Ok(jar) = std::env::var("KOTLINX_COROUTINES_JAR") {
        return Some(PathBuf::from(jar));
    }

    let home = PathBuf::from(std::env::var("HOME").ok()?);
    [
        home.join(
            ".gradle/caches/modules-2/files-2.1/org.jetbrains.kotlinx/kotlinx-coroutines-core-jvm",
        ),
        home.join(".m2/repository/org/jetbrains/kotlinx/kotlinx-coroutines-core-jvm"),
    ]
    .iter()
    .find_map(|root| find_jar(root, 4))
}

/// The first jar under `dir`, no deeper than `depth`, without the `-sources`
/// jars.
fn find_jar(dir: &Path, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }

    let mut entries: Vec<_> = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();

    entries.iter().find_map(|path| {
        let name = path.file_name()?.to_str()?;
        if path.is_dir() {
            find_jar(path, depth - 1)
        } else {
            let jar = path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"));
            (jar && !name.ends_with("-sources.jar")).then(|| path.clone())
        }
    })
}

#[test]
fn the_swift_protocol_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .swift(&Config::builder("App", dir.path()).build())
        .expect("swift type generation should succeed");

    let protocol = fs::read_to_string(dir.path().join("App/Sources/App").join(SWIFT.0))
        .expect("should write the protocol beside the module");
    assert!(
        protocol.contains(SWIFT.1),
        "unexpected protocol:\n{protocol}"
    );

    build("swift", &["build"], &dir.path().join("App"));
}

#[test]
fn the_kotlin_protocol_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .kotlin(&Config::builder("com.example.shared", dir.path()).build())
        .expect("kotlin type generation should succeed");

    let protocol = fs::read_to_string(dir.path().join("com/example/shared").join(KOTLIN.0))
        .expect("should write the protocol beside the module");
    assert!(protocol.starts_with("package com.example.shared\n"));
    assert!(
        protocol.contains(KOTLIN.1),
        "unexpected protocol:\n{protocol}"
    );

    let Some(coroutines) = coroutines_jar() else {
        unavailable(
            "there is no kotlinx-coroutines-core jar in KOTLINX_COROUTINES_JAR or under \
             ~/.gradle or ~/.m2, and `kotlinc` ships only the standard library",
            "the generated protocol is not compiled",
        );
        return;
    };

    // The package directories, not the package root, because `kotlinc` would
    // then try to compile the `build.gradle.kts` file in it.
    let mut args: Vec<OsString> = fs::read_dir(dir.path())
        .expect("should read the generated package")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(PathBuf::into_os_string)
        .collect();
    args.sort();
    args.extend([
        "-classpath".into(),
        coroutines.into_os_string(),
        "-d".into(),
        dir.path().join("classes.jar").into_os_string(),
    ]);
    build("kotlinc", &args, dir.path());
}

/// `typescript()` runs `tsc` but does not check its exit status, so this test
/// runs `tsc` again and checks it.
#[test]
fn the_typescript_protocol_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .typescript(&Config::builder("shared_types", dir.path()).build())
        .expect("typescript type generation should succeed");

    let module = fs::read_to_string(dir.path().join("shared_types.ts"))
        .expect("should write a TypeScript module");
    assert!(module.contains(TYPESCRIPT), "unexpected module:\n{module}");

    build("pnpm", &["exec", "tsc", "--build", "--force"], dir.path());
}

#[test]
fn the_csharp_protocol_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .csharp(&Config::builder("Example.Shared", dir.path()).build())
        .expect("c# type generation should succeed");

    let protocol = fs::read_to_string(dir.path().join("Example/Shared").join(CSHARP.0))
        .expect("should write the protocol beside the module");
    assert!(
        protocol.contains(CSHARP.1),
        "unexpected protocol:\n{protocol}"
    );

    build("dotnet", &["build", "--nologo"], dir.path());
}

/// The protocol names every operation of the capability, so the app gets the
/// types of the operations that it does not send.
#[test]
fn the_app_gets_every_type_of_the_capability() {
    let registry = generator().registry();
    assert!(registry.keys().any(|name| name.name == UNSENT));
}
